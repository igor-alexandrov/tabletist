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

/// The zone a database server names `name` (MySQL's session zone):
/// `UTC`, an offset such as `+00:00` or `-05:30`, or a zone of the
/// database by its name. `None` for anything else, an abbreviation such
/// as `CET` or `EST` above all: it names no rules of the clocks, and the
/// zone of the database by that name is not the server's. Disk work, for
/// the backend.
pub fn session(name: &str) -> Option<TimeZone> {
    let name = name.trim();
    if name.eq_ignore_ascii_case("utc") || name.eq_ignore_ascii_case("gmt") {
        return Some(TimeZone::UTC);
    }
    if let Some(sign) = name.strip_prefix(['+', '-']).map(|_| &name[..1]) {
        let (hours, minutes) = name[1..].split_once(':')?;
        let (hours, minutes) = (hours.parse::<i32>().ok()?, minutes.parse::<i32>().ok()?);
        let seconds = (hours * 60 + minutes) * 60 * if sign == "-" { -1 } else { 1 };
        return Some(TimeZone::fixed(
            jiff::tz::Offset::from_seconds(seconds).ok()?,
        ));
    }
    if !name.contains('/') {
        return None;
    }
    jiff::tz::db().get(name).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_servers_zone_is_read_where_it_names_rules_or_an_offset() {
        assert_eq!(session("UTC"), Some(TimeZone::UTC));
        let berlin = session("Europe/Berlin").expect("a zone");
        assert_eq!(berlin.iana_name(), Some("Europe/Berlin"));
        let offset = |seconds| {
            Some(TimeZone::fixed(
                jiff::tz::Offset::from_seconds(seconds).unwrap(),
            ))
        };
        assert_eq!(session("+00:00"), offset(0));
        assert_eq!(session("-05:30"), offset(-19_800));
        assert_eq!(session("CET"), None);
        assert_eq!(session("EST"), None);
        assert_eq!(session("Mars/Olympus"), None);
    }

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
