//! TLS for database connections on rustls, with libpq's `sslmode` meanings:
//! `prefer` and `require` encrypt without checking the certificate,
//! `verify-ca` checks the chain, `verify-full` checks the chain and the host.
//! `verify-ca` needs a CA file: any publicly trusted certificate chains to
//! the system roots, so without the host check those prove nothing (libpq
//! likewise turns `sslrootcert=system` into `verify-full`).

use std::path::Path;
use std::sync::Arc;

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, SignatureScheme};

use crate::{Error, Result, TlsMode};

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn tls_error(error: impl std::fmt::Display) -> Error {
    Error::Tls(error.to_string())
}

/// Why `verify-ca` without a CA file is refused.
const VERIFY_CA_NEEDS_A_CA_FILE: &str = "Verify certificate needs a CA file; use Verify certificate and host to use the system certificates";

/// The rustls configuration for `mode`. `Disable` still gets one (the
/// connector needs it) but it is never used.
pub(crate) fn client_config(mode: TlsMode, ca_file: Option<&Path>) -> Result<rustls::ClientConfig> {
    let provider = provider();
    let builder = rustls::ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .map_err(tls_error)?;
    let config = match mode {
        TlsMode::Disable | TlsMode::Prefer | TlsMode::Require => builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerify(provider)))
            .with_no_client_auth(),
        TlsMode::VerifyCa => {
            let ca_file = ca_file.ok_or_else(|| Error::Tls(VERIFY_CA_NEEDS_A_CA_FILE.into()))?;
            let verifier =
                WebPkiServerVerifier::builder_with_provider(roots(Some(ca_file))?, provider)
                    .build()
                    .map_err(tls_error)?;
            builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(IgnoreName(verifier)))
                .with_no_client_auth()
        }
        TlsMode::VerifyFull => builder
            .with_root_certificates(roots(ca_file)?)
            .with_no_client_auth(),
    };
    Ok(config)
}

/// The CA file's certificates, or the operating system's roots.
fn roots(ca_file: Option<&Path>) -> Result<Arc<rustls::RootCertStore>> {
    let mut roots = rustls::RootCertStore::empty();
    match ca_file {
        Some(path) => {
            let certificates = CertificateDer::pem_file_iter(path).map_err(|error| {
                Error::Tls(format!("could not read {}: {error}", path.display()))
            })?;
            for certificate in certificates {
                let certificate = certificate.map_err(tls_error)?;
                roots.add(certificate).map_err(tls_error)?;
            }
            if roots.is_empty() {
                return Err(Error::Tls(format!(
                    "{} holds no certificates",
                    path.display()
                )));
            }
        }
        None => {
            for certificate in rustls_native_certs::load_native_certs().certs {
                let _ = roots.add(certificate);
            }
            if roots.is_empty() {
                return Err(Error::Tls(
                    "no trusted certificates found on this system".into(),
                ));
            }
        }
    }
    Ok(Arc::new(roots))
}

/// libpq's `sslmode` for our mode. Certificate checks happen in rustls, so
/// every verifying mode is simply "require TLS" to tokio-postgres.
pub(crate) fn ssl_mode(mode: TlsMode) -> tokio_postgres::config::SslMode {
    use tokio_postgres::config::SslMode;
    match mode {
        TlsMode::Disable => SslMode::Disable,
        TlsMode::Prefer => SslMode::Prefer,
        TlsMode::Require | TlsMode::VerifyCa | TlsMode::VerifyFull => SslMode::Require,
    }
}

/// Accepts any certificate but still checks handshake signatures:
/// `prefer` and `require` in libpq do not verify the server.
#[derive(Debug)]
pub(crate) struct NoVerify(Arc<CryptoProvider>);

impl ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Checks the certificate chain but not the host name (`verify-ca`).
#[derive(Debug)]
pub(crate) struct IgnoreName(pub(crate) Arc<dyn ServerCertVerifier>);

impl ServerCertVerifier for IgnoreName {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        match self
            .0
            .verify_server_cert(end_entity, intermediates, server_name, ocsp, now)
        {
            Err(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.0
            .verify_tls12_signature(message, certificate, signature)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.0
            .verify_tls13_signature(message, certificate, signature)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_verify_schemes()
    }
}

/// Whether a rustls error is anywhere in the chain (wrapped in io::Error by
/// tokio-rustls).
pub(crate) fn is_tls_error(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(error) = current {
        if error.downcast_ref::<rustls::Error>().is_some() {
            return true;
        }
        if let Some(io) = error.downcast_ref::<std::io::Error>()
            && io
                .get_ref()
                .is_some_and(|inner| inner.downcast_ref::<rustls::Error>().is_some())
        {
            return true;
        }
        current = error.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};

    #[test]
    fn every_mode_builds_a_config() {
        for mode in [TlsMode::Disable, TlsMode::Prefer, TlsMode::Require] {
            assert!(client_config(mode, None).is_ok(), "{mode:?}");
        }
    }

    #[test]
    fn verify_ca_without_a_ca_file_is_refused() {
        match client_config(TlsMode::VerifyCa, None) {
            Err(Error::Tls(message)) => assert_eq!(message, VERIFY_CA_NEEDS_A_CA_FILE),
            other => panic!("expected a TLS error, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn verify_ca_with_a_ca_file_builds_a_config() {
        let dir = tempfile::tempdir().unwrap();
        let ca = dir.path().join("ca.pem");
        // A self-signed CA certificate; its key was thrown away.
        std::fs::write(&ca, include_str!("../tests/tls/ca.pem")).unwrap();
        assert!(client_config(TlsMode::VerifyCa, Some(&ca)).is_ok());
    }

    #[test]
    fn a_missing_or_empty_ca_file_is_a_tls_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.pem");
        assert!(matches!(
            client_config(TlsMode::VerifyFull, Some(&missing)),
            Err(Error::Tls(_))
        ));
        let empty = dir.path().join("empty.pem");
        std::fs::write(&empty, "no certificates here").unwrap();
        assert!(matches!(
            client_config(TlsMode::VerifyCa, Some(&empty)),
            Err(Error::Tls(_))
        ));
    }

    #[test]
    fn modes_map_to_sslmode() {
        use tokio_postgres::config::SslMode;
        assert!(matches!(ssl_mode(TlsMode::Disable), SslMode::Disable));
        assert!(matches!(ssl_mode(TlsMode::Prefer), SslMode::Prefer));
        for mode in [TlsMode::Require, TlsMode::VerifyCa, TlsMode::VerifyFull] {
            assert!(matches!(ssl_mode(mode), SslMode::Require), "{mode:?}");
        }
    }

    /// A verifier that fails every certificate with a fixed error.
    #[derive(Debug)]
    struct Failing(rustls::Error);

    impl ServerCertVerifier for Failing {
        fn verify_server_cert(
            &self,
            _: &CertificateDer<'_>,
            _: &[CertificateDer<'_>],
            _: &ServerName<'_>,
            _: &[u8],
            _: UnixTime,
        ) -> std::result::Result<ServerCertVerified, rustls::Error> {
            Err(self.0.clone())
        }
        fn verify_tls12_signature(
            &self,
            _: &[u8],
            _: &CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
            Err(self.0.clone())
        }
        fn verify_tls13_signature(
            &self,
            _: &[u8],
            _: &CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
            Err(self.0.clone())
        }
        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            Vec::new()
        }
    }

    fn verify(verifier: &dyn ServerCertVerifier) -> bool {
        let name = ServerName::try_from("db.example.com").unwrap();
        verifier
            .verify_server_cert(
                &CertificateDer::from(vec![1, 2, 3]),
                &[],
                &name,
                &[],
                UnixTime::now(),
            )
            .is_ok()
    }

    #[test]
    fn verify_ca_ignores_only_a_name_mismatch() {
        let name = IgnoreName(Arc::new(Failing(rustls::Error::InvalidCertificate(
            CertificateError::NotValidForName,
        ))));
        assert!(verify(&name));
        let issuer = IgnoreName(Arc::new(Failing(rustls::Error::InvalidCertificate(
            CertificateError::UnknownIssuer,
        ))));
        assert!(!verify(&issuer));
    }

    #[test]
    fn no_verify_accepts_any_certificate() {
        assert!(verify(&NoVerify(provider())));
    }

    /// mysql_async builds its TLS config with rustls's process-default
    /// provider, which rustls only picks by itself when exactly one provider
    /// (ring) is compiled in. Enabling aws-lc anywhere would make MySQL TLS
    /// connections panic.
    #[test]
    fn exactly_one_crypto_provider_is_compiled_in() {
        let config = std::panic::catch_unwind(|| {
            rustls::ClientConfig::builder()
                .with_root_certificates(rustls::RootCertStore::empty())
                .with_no_client_auth()
        });
        assert!(
            config.is_ok(),
            "rustls could not choose a provider by itself"
        );
    }
}
