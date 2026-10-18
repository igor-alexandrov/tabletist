//! The time zone database, read once on the backend (`Command::LoadZones`)
//! and handed to the app whole: the names the zone menu lists, and each
//! zone a `timestamptz` can be shown in. On Linux and macOS the database is
//! the system's `/usr/share/zoneinfo`, a file a zone: the UI thread reads
//! none of it.

use std::collections::HashMap;

use jiff::tz::TimeZone;

/// Every zone of the database, by name.
#[derive(Clone, Default)]
pub struct Zones {
    /// In alphabetical order, each once.
    names: Vec<String>,
    /// By the name in lower case: a name is found whatever its case, as
    /// the database finds it.
    zones: HashMap<String, TimeZone>,
}

impl std::fmt::Debug for Zones {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Zones({} names)", self.names.len())
    }
}

impl Zones {
    /// Reads the whole database: its names, and each zone. Disk work, for
    /// the backend.
    pub fn load() -> Self {
        let db = jiff::tz::db();
        let mut names: Vec<String> = db.available().map(|name| name.to_string()).collect();
        names.sort_unstable();
        names.dedup();
        let zones = names
            .iter()
            .filter_map(|name| Some((name.to_lowercase(), db.get(name).ok()?)))
            .collect();
        Self { names, zones }
    }

    /// The zones' names, in alphabetical order.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// The zone `name` names, whatever its case. `None` for a name the
    /// database does not know.
    pub fn get(&self, name: &str) -> Option<&TimeZone> {
        self.zones.get(&name.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_database_is_read_whole_and_found_in_any_case() {
        let zones = Zones::load();
        assert!(zones.names().len() > 100);
        assert!(zones.names().windows(2).all(|pair| pair[0] < pair[1]));
        let new_york = zones.get("America/New_York").expect("a zone");
        assert_eq!(new_york.iana_name(), Some("America/New_York"));
        assert!(zones.get("america/new_york").is_some());
        assert!(zones.get("Mars/Olympus").is_none());
    }
}
