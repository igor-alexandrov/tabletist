//! What a restart to install an update keeps that the session does not:
//! each window's open SQL editors, by their title's number, with their
//! text, their mode and their place among the window's tabs, and which
//! tab was in front. Written to `relaunch.json` in the state folder as the
//! handoff begins, and read by the copy the helper starts (`--relaunch`),
//! which deletes it then. Deleted too where the handoff does not end in
//! the quit. A quit of any other kind writes none.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// The file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relaunch {
    /// The windows, in the order the bar had them.
    #[serde(default)]
    pub windows: Vec<Window>,
}

/// One window's SQL editors, in the order its tabs had them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    /// The saved connection's id.
    pub connection: String,
    #[serde(default)]
    pub editors: Vec<Editor>,
    /// The tab in front, by its place among the window's tables and
    /// editors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front: Option<usize>,
}

/// A SQL editor: Query 2, its text, whether its runs may write, and its
/// place among the window's tables and editors.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Editor {
    pub number: u32,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub writes: bool,
    #[serde(default)]
    pub at: usize,
}

// Hand-written so the SQL text never reaches logs or panic messages.
impl std::fmt::Debug for Editor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Editor")
            .field("number", &self.number)
            .field("text", &format_args!("{} bytes", self.text.len()))
            .field("writes", &self.writes)
            .field("at", &self.at)
            .finish()
    }
}

impl Relaunch {
    /// The file at `path`, deleted once read: it is for one launch. An
    /// empty one where there is none, or it cannot be read.
    pub fn take(path: &Path) -> Self {
        let read = crate::util::load_json(path);
        if let Err(error) = std::fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!("could not delete {}: {error}", path.display());
        }
        read
    }

    /// The first window of `connection` not taken yet, taken: windows
    /// asked for in the bar's order each get their own, where a
    /// connection is open in two.
    pub fn window_of(&mut self, connection: &str) -> Option<Window> {
        let at = self
            .windows
            .iter()
            .position(|window| window.connection == connection)?;
        Some(self.windows.remove(at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept() -> Relaunch {
        Relaunch {
            windows: vec![
                Window {
                    connection: "bookshop".into(),
                    editors: vec![Editor {
                        number: 2,
                        text: "SELECT * FROM book_covers".into(),
                        ..Editor::default()
                    }],
                    front: None,
                },
                Window {
                    connection: "bookshop".into(),
                    editors: vec![Editor {
                        number: 1,
                        text: "SELECT 1".into(),
                        ..Editor::default()
                    }],
                    front: None,
                },
            ],
        }
    }

    #[test]
    fn the_file_is_read_once_and_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("relaunch.json");
        crate::util::save_json(&path, &kept()).unwrap();
        assert_eq!(Relaunch::take(&path), kept());
        assert!(!path.exists());
        assert_eq!(Relaunch::take(&path), Relaunch::default());
    }

    #[test]
    fn each_window_of_a_connection_takes_its_own_editors() {
        let mut kept = kept();
        let number = |window: Option<Window>| window.map(|window| window.editors[0].number);
        assert_eq!(number(kept.window_of("bookshop")), Some(2));
        assert_eq!(number(kept.window_of("bookshop")), Some(1));
        assert!(kept.window_of("bookshop").is_none());
    }

    /// A file a newer version wrote, with more in it, is read for what
    /// this one knows.
    #[test]
    fn what_this_version_does_not_know_is_left() {
        let text = r#"{"windows":[{"connection":"bookshop","editors":[{"number":3,"text":"SELECT 2","mode":"write"}],"front":1}],"version":2}"#;
        let read: Relaunch = serde_json::from_str(text).unwrap();
        assert_eq!(read.windows[0].editors[0].number, 3);
    }
}
