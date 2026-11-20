# ClickHouse's test certificate

HTTPS on the compose server's 8443 (`tls.xml`), for the TLS tests in
`crates/tabletist-db`. Public test material: never trust it elsewhere.

- `ca.pem`: a test CA, valid for 100 years. Its key was thrown away.
- `server.pem`, `server.key`: the server's certificate, signed by it, for
  `localhost`, `clickhouse` (the compose network's name, which a test
  reaches through the SSH service) and `127.0.0.1`.

Made with OpenSSL 3, from this directory:

```bash
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -keyout ca.key -out ca.pem -days 36500 -subj "/CN=Tabletist test CA" \
  -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign"
openssl req -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -keyout server.key -out server.csr -subj "/CN=localhost"
printf 'subjectAltName=DNS:localhost,DNS:clickhouse,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\n' > ext.cnf
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out server.pem -days 36500 -extfile ext.cnf
rm ca.key ca.srl server.csr ext.cnf
chmod 644 server.key
```
