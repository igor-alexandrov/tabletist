//! Saved passwords in the OS keyring (Secret Service on Linux, Keychain on
//! macOS, Credential Manager on Windows). Every call runs on one dedicated
//! thread, so a locked keyring waiting for the user never blocks the UI or
//! the database runtime.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use crate::connections::ConnectionId;

/// The keyring service all of Tabletist's entries live under.
pub const SERVICE: &str = "dev.tabletist.Tabletist";
const TIMEOUT: Duration = Duration::from_secs(20);

/// The keyring account for a saved connection's password.
pub fn password_account(id: &ConnectionId) -> String {
    format!("connection/{}/password", id.0)
}

/// The keyring account for a connection's SSH password or key passphrase.
pub fn ssh_account(id: &ConnectionId) -> String {
    format!("connection/{}/ssh-secret", id.0)
}

/// A password in transit. Prints as `SecretString(..)`.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(pub String);

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(..)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// The keyring is locked or refused access.
    Locked,
    /// No keyring, or it failed.
    Unavailable,
    /// The keyring did not answer in time (often an unlock prompt left open).
    TimedOut,
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Locked => "the keyring is locked",
            Self::Unavailable => "the keyring is not available",
            Self::TimedOut => "the keyring did not answer",
        })
    }
}

pub trait SecretStore: Send {
    fn read(&mut self, account: &str) -> Result<Option<String>, SecretError>;
    /// Reads without asking the person for anything: where the store would
    /// have to be unlocked first, it answers [`SecretError::Locked`] and
    /// asks nothing. For a read nobody at the screen asked for.
    fn read_quietly(&mut self, account: &str) -> Result<Option<String>, SecretError> {
        self.read(account)
    }
    fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError>;
    fn delete(&mut self, account: &str) -> Result<(), SecretError>;
}

/// For tests and demo mode.
#[derive(Default)]
pub struct MemoryStore(HashMap<String, String>);

impl SecretStore for MemoryStore {
    fn read(&mut self, account: &str) -> Result<Option<String>, SecretError> {
        Ok(self.0.get(account).cloned())
    }

    fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError> {
        self.0.insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&mut self, account: &str) -> Result<(), SecretError> {
        self.0.remove(account);
        Ok(())
    }
}

/// The platform keyring, opened on first use.
#[derive(Default)]
pub struct NativeStore {
    store: Option<Arc<keyring_core::api::CredentialStore>>,
}

/// Provider errors can quote platform data; never pass their text on.
fn native_error(error: keyring_core::Error) -> SecretError {
    match error {
        keyring_core::Error::NoStorageAccess(_) => SecretError::Locked,
        _ => SecretError::Unavailable,
    }
}

impl NativeStore {
    fn entry(&mut self, account: &str) -> Result<keyring_core::Entry, SecretError> {
        if self.store.is_none() {
            #[cfg(target_os = "linux")]
            let store = zbus_secret_service_keyring_store::Store::new();
            #[cfg(target_os = "macos")]
            let store = apple_native_keyring_store::keychain::Store::new();
            #[cfg(windows)]
            let store = windows_native_keyring_store::Store::new();
            self.store = Some(store.map_err(native_error)?);
        }
        self.store
            .as_ref()
            .ok_or(SecretError::Unavailable)?
            .build(SERVICE, account, None)
            .map_err(native_error)
    }
}

impl SecretStore for NativeStore {
    fn read(&mut self, account: &str) -> Result<Option<String>, SecretError> {
        match self.entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(native_error(error)),
        }
    }

    /// On Linux the Secret Service asks the person to unlock a collection
    /// whose item a read finds locked: so the item is looked for first,
    /// which asks nothing, and read only when it was found unlocked. One
    /// that was not found is not read either: should the search look for
    /// it otherwise than the keyring crate keeps it, the cost is a
    /// password not found, never a prompt. Windows asks nothing of a read.
    /// On macOS a locked keychain is asked to be unlocked by the system,
    /// and no call of the keyring crate says not to: that is not kept from
    /// happening here.
    fn read_quietly(&mut self, account: &str) -> Result<Option<String>, SecretError> {
        #[cfg(target_os = "linux")]
        match linux_item(account)? {
            Item::Unlocked => {}
            Item::Locked => return Err(SecretError::Locked),
            Item::Missing => return Ok(None),
        }
        self.read(account)
    }

    fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError> {
        self.entry(account)?
            .set_password(secret)
            .map_err(native_error)
    }

    fn delete(&mut self, account: &str) -> Result<(), SecretError> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(native_error(error)),
        }
    }
}

/// How the Secret Service holds an account's item.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    /// Every item the search found is unlocked: a read asks nothing.
    Unlocked,
    /// One at least is locked: a read would ask to unlock it.
    Locked,
    /// None was found.
    Missing,
}

/// How the Secret Service holds the account's item. Searching asks the
/// person nothing, and sends no secret.
#[cfg(target_os = "linux")]
fn linux_item(account: &str) -> Result<Item, SecretError> {
    use secret_service::EncryptionType;
    use secret_service::blocking::SecretService;
    let service =
        SecretService::connect(EncryptionType::Plain).map_err(|_| SecretError::Unavailable)?;
    // The attributes `zbus-secret-service-keyring-store` gives an entry it
    // makes without a target (its `Specifier::search_attributes`).
    let found = service
        .search_items(HashMap::from([("service", SERVICE), ("username", account)]))
        .map_err(|_| SecretError::Unavailable)?;
    Ok(item(found.locked.len(), found.unlocked.len()))
}

/// What a search's locked and unlocked counts say of an item.
#[cfg(target_os = "linux")]
fn item(locked: usize, unlocked: usize) -> Item {
    match (locked, unlocked) {
        (0, 0) => Item::Missing,
        (0, _) => Item::Unlocked,
        _ => Item::Locked,
    }
}

type Job = Box<dyn FnOnce(&mut dyn SecretStore) + Send>;

/// A handle to the keyring thread.
#[derive(Clone)]
pub struct Keyring {
    jobs: mpsc::Sender<Job>,
    /// Whether this is the OS keyring (not an in-memory store).
    native: bool,
}

/// An answer that will arrive from the keyring thread.
pub struct Pending<T>(tokio::sync::oneshot::Receiver<Result<T, SecretError>>);

impl<T> Pending<T> {
    pub async fn wait(self) -> Result<T, SecretError> {
        match tokio::time::timeout(TIMEOUT, self.0).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(SecretError::Unavailable),
            Err(_) => Err(SecretError::TimedOut),
        }
    }
}

impl Keyring {
    pub fn start(store: impl SecretStore + 'static) -> Self {
        Self::spawn(store, false)
    }

    fn spawn(store: impl SecretStore + 'static, native: bool) -> Self {
        let (jobs, queue) = mpsc::channel::<Job>();
        let mut store = store;
        let spawned = std::thread::Builder::new()
            .name("tabletist-keyring".into())
            .spawn(move || {
                while let Ok(job) = queue.recv() {
                    job(&mut store);
                }
            });
        if let Err(error) = spawned {
            log::error!("could not start the keyring thread: {error}");
        }
        Self { jobs, native }
    }

    pub fn native() -> Self {
        Self::spawn(NativeStore::default(), true)
    }

    pub fn memory() -> Self {
        Self::start(MemoryStore::default())
    }

    /// Whether secrets go to the OS keyring rather than memory.
    pub fn is_native(&self) -> bool {
        self.native
    }

    /// Queues `work` now (so calls run in order) and returns its answer.
    fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut dyn SecretStore) -> Result<T, SecretError> + Send + 'static,
    ) -> Pending<T> {
        let (answer, pending) = tokio::sync::oneshot::channel();
        let job: Job = Box::new(move |store| {
            let _ = answer.send(work(store));
        });
        // A failed send drops the job; `wait` then reports Unavailable.
        let _ = self.jobs.send(job);
        Pending(pending)
    }

    pub fn read(&self, account: &str) -> Pending<Option<String>> {
        let account = account.to_owned();
        self.run(move |store| store.read(&account))
    }

    /// [`SecretStore::read_quietly`]: a read nobody at the screen asked for.
    pub fn read_quietly(&self, account: &str) -> Pending<Option<String>> {
        let account = account.to_owned();
        self.run(move |store| store.read_quietly(&account))
    }

    pub fn write(&self, account: &str, secret: SecretString) -> Pending<()> {
        let account = account.to_owned();
        self.run(move |store| store.write(&account, &secret.0))
    }

    pub fn delete(&self, account: &str) -> Pending<()> {
        let account = account.to_owned();
        self.run(move |store| store.delete(&account))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn a_quiet_read_reads_only_an_item_it_found_unlocked() {
        assert_eq!(item(0, 1), Item::Unlocked);
        assert_eq!(item(0, 2), Item::Unlocked);
        // A read would ask to unlock the locked one, whatever else it found.
        assert_eq!(item(1, 0), Item::Locked);
        assert_eq!(item(1, 1), Item::Locked);
        // Not found, perhaps by a search that looked otherwise than the
        // keyring crate keeps it: not read, so asked nothing.
        assert_eq!(item(0, 0), Item::Missing);
    }

    #[tokio::test]
    async fn a_quiet_read_of_a_store_that_never_asks_is_a_read() {
        let keyring = Keyring::memory();
        assert_eq!(keyring.read_quietly("a").wait().await, Ok(None));
        keyring
            .write("a", SecretString("hunter2".into()))
            .wait()
            .await
            .unwrap();
        assert_eq!(
            keyring.read_quietly("a").wait().await,
            Ok(Some("hunter2".into()))
        );
    }

    #[tokio::test]
    async fn the_memory_keyring_stores_reads_and_deletes() {
        let keyring = Keyring::memory();
        assert_eq!(keyring.read("a").wait().await, Ok(None));
        keyring
            .write("a", SecretString("hunter2".into()))
            .wait()
            .await
            .unwrap();
        assert_eq!(keyring.read("a").wait().await, Ok(Some("hunter2".into())));
        keyring.delete("a").wait().await.unwrap();
        keyring.delete("a").wait().await.unwrap();
        assert_eq!(keyring.read("a").wait().await, Ok(None));
    }

    #[tokio::test]
    async fn jobs_run_in_the_order_they_were_asked_for() {
        let keyring = Keyring::memory();
        let write = keyring.write("a", SecretString("first".into()));
        let read = keyring.read("a");
        // Waiting in the other order still sees the write.
        assert_eq!(read.wait().await, Ok(Some("first".into())));
        write.wait().await.unwrap();
    }

    #[test]
    fn secrets_do_not_print() {
        assert!(!format!("{:?}", SecretString("hunter2".into())).contains("hunter2"));
    }

    #[test]
    fn accounts_are_per_connection() {
        let id = crate::connections::ConnectionId("abc".into());
        assert_eq!(password_account(&id), "connection/abc/password");
    }

    /// Talks to the real OS keyring; run by hand or in CI on macOS/Windows:
    /// `cargo test --lib secrets::tests::native_store_round_trip -- --ignored --exact`
    #[tokio::test]
    #[ignore]
    async fn native_store_round_trip() {
        let keyring = Keyring::native();
        let account = format!("test/{}", std::process::id());
        keyring
            .write(&account, SecretString("dummy".into()))
            .wait()
            .await
            .unwrap();
        assert_eq!(
            keyring.read(&account).wait().await,
            Ok(Some("dummy".into()))
        );
        keyring.delete(&account).wait().await.unwrap();
        assert_eq!(keyring.read(&account).wait().await, Ok(None));
    }
}
