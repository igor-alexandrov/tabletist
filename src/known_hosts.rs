//! Trusted SSH host keys, saved in config/known_hosts.json.

use std::path::Path;

use tabletist_db::HostKeys;

/// The store, or why it cannot be read. A missing file is an empty store;
/// a damaged one is an error and is left in place, so the keys it holds are
/// never silently forgotten (which would let a changed key look new).
pub fn load(path: &Path) -> Result<HostKeys, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(HostKeys::default()),
        Err(error) => Err(error.to_string()),
    }
}

pub fn save(path: &Path, keys: &HostKeys) -> std::io::Result<()> {
    crate::util::save_json(path, keys)
}
