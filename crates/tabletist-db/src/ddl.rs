//! A table's or a view's definition as text, for the DDL view: what
//! PostgreSQL's catalog builds, and what MySQL and SQLite keep.

use crate::Driver;

/// What a part of a definition holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartKind {
    /// The `CREATE` of the table or the view itself.
    Object,
    Indexes,
    Triggers,
    Comments,
}

impl PartKind {
    /// The comment over the part where the text puts several parts
    /// together: SQL, so never translated.
    fn heading(self) -> &'static str {
        match self {
            Self::Object => "",
            Self::Indexes => "-- Indexes",
            Self::Triggers => "-- Triggers",
            Self::Comments => "-- Comments",
        }
    }
}

/// One part of a definition and its statements, each as the database gave
/// it or as the catalog built it. A trigger is one statement, with the
/// `ALTER TABLE` after it where it is not enabled as a new trigger is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub kind: PartKind,
    /// At least one: a part with nothing in it is left out.
    pub statements: Vec<String>,
}

/// A table's or a view's definition, by its parts, so that what lists the
/// parts and the text come from the same data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ddl {
    /// Whose text it is: each joins its parts its own way.
    pub driver: Driver,
    /// The server it was read from, as the footer of a SQL editor names
    /// it: `PostgreSQL 17.2`, `MySQL 8.4.3`, `SQLite 3.46.0`.
    pub server: String,
    /// The comment the text starts with, without its `-- `: what the text
    /// is of and where it comes from. PostgreSQL's only, whose text the
    /// app builds.
    pub heading: Option<String>,
    /// The object's part first, then the others in the order the text
    /// gives them.
    pub parts: Vec<Part>,
}

/// Where a definition's text comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Built from the catalog: PostgreSQL has no `SHOW CREATE TABLE`.
    Catalog,
    /// As the database keeps it, and returns it.
    Stored,
}

impl Ddl {
    /// The parts, the object's first, with the empty ones left out.
    pub(crate) fn new(
        driver: Driver,
        server: String,
        heading: Option<String>,
        parts: Vec<Part>,
    ) -> Self {
        let parts = parts
            .into_iter()
            .filter(|part| !part.statements.is_empty())
            .collect();
        Self {
            driver,
            server,
            heading,
            parts,
        }
    }

    pub fn source(&self) -> Source {
        match self.driver {
            Driver::Postgres => Source::Catalog,
            Driver::MySql | Driver::Sqlite => Source::Stored,
        }
    }

    /// The part of `kind`, where it holds anything.
    pub fn part(&self, kind: PartKind) -> Option<&Part> {
        self.parts.iter().find(|part| part.kind == kind)
    }

    /// The whole text, each statement ending in `;`. PostgreSQL's starts
    /// with its heading, and it and MySQL's put a blank line and a comment
    /// over each part after the object's. SQLite's keeps its statements
    /// apart by a blank line, and adds nothing else.
    pub fn text(&self) -> String {
        self.laid_out().0
    }

    /// [`Self::text`], and the lines each part takes in it, from nought:
    /// from its comment (the object's from the first line, its heading's)
    /// to its last statement's last line, the blank line after it left out.
    /// What lists the parts and the text are made together, so they never
    /// disagree.
    pub fn laid_out(&self) -> (String, Vec<(PartKind, std::ops::Range<usize>)>) {
        let line = |text: &str| text.matches('\n').count();
        let mut placed = Vec::with_capacity(self.parts.len());
        let mut text = String::new();
        if let Some(heading) = &self.heading {
            text.push_str("-- ");
            text.push_str(heading);
            text.push('\n');
        }
        let sections = match self.driver {
            Driver::Postgres | Driver::MySql => true,
            Driver::Sqlite => false,
        };
        let mut written = false;
        for part in &self.parts {
            let mut start = None;
            for (index, statement) in part.statements.iter().enumerate() {
                let first = index == 0;
                if written {
                    // A part's statements are one under the other.
                    text.push_str(if sections && !first { "\n" } else { "\n\n" });
                }
                // The object's part starts at the heading, at the top.
                let from = if part.kind == PartKind::Object {
                    0
                } else {
                    line(&text)
                };
                start.get_or_insert(from);
                if sections && first && part.kind != PartKind::Object {
                    text.push_str(part.kind.heading());
                    text.push('\n');
                }
                text.push_str(&terminated(statement));
                written = true;
            }
            if let Some(start) = start {
                placed.push((part.kind, start..line(&text) + 1));
            }
        }
        (text, placed)
    }
}

/// `statement` ending in `;`, as the text joins it: the one it has, or one
/// added after it. Where the last line holds a `--` comment, which would
/// take in a `;` after it (or holds one already), it goes on a line of its
/// own.
fn terminated(statement: &str) -> String {
    let statement = statement.trim_end();
    let last = statement.lines().last().unwrap_or_default();
    if last.contains("--") {
        format!("{statement}\n;")
    } else if statement.ends_with(';') {
        statement.to_owned()
    } else {
        format!("{statement};")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(kind: PartKind, statements: &[&str]) -> Part {
        Part {
            kind,
            statements: statements.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn postgres_text_starts_with_its_heading_and_names_each_part() {
        let ddl = Ddl::new(
            Driver::Postgres,
            "PostgreSQL 17.2".into(),
            Some("public.t · built from the catalog · PostgreSQL 17.2".into()),
            vec![
                part(
                    PartKind::Object,
                    &["CREATE TABLE public.t (\n    id integer\n);"],
                ),
                part(
                    PartKind::Indexes,
                    &[
                        "CREATE INDEX a ON public.t (id)",
                        "CREATE INDEX b ON public.t (id)",
                    ],
                ),
                part(PartKind::Triggers, &[]),
                part(PartKind::Comments, &["COMMENT ON TABLE public.t IS 'x';"]),
            ],
        );
        assert_eq!(
            ddl.text(),
            "-- public.t · built from the catalog · PostgreSQL 17.2\n\
             CREATE TABLE public.t (\n    id integer\n);\n\
             \n\
             -- Indexes\n\
             CREATE INDEX a ON public.t (id);\n\
             CREATE INDEX b ON public.t (id);\n\
             \n\
             -- Comments\n\
             COMMENT ON TABLE public.t IS 'x';"
        );
        // The empty part is no part.
        assert_eq!(ddl.part(PartKind::Triggers), None);
        // Each part's lines, from its comment to its last line.
        assert_eq!(
            ddl.laid_out().1,
            [
                (PartKind::Object, 0..4),
                (PartKind::Indexes, 5..8),
                (PartKind::Comments, 9..11),
            ]
        );
        assert_eq!(ddl.source(), Source::Catalog);
    }

    #[test]
    fn mysql_text_puts_its_triggers_under_a_comment() {
        let ddl = Ddl::new(
            Driver::MySql,
            "MySQL 8.4.3".into(),
            None,
            vec![
                part(
                    PartKind::Object,
                    &["CREATE TABLE `t` (\n  `id` int\n) ENGINE=InnoDB"],
                ),
                part(
                    PartKind::Triggers,
                    &["CREATE TRIGGER `a` BEFORE INSERT ON `t` FOR EACH ROW SET NEW.id = 1"],
                ),
            ],
        );
        assert_eq!(
            ddl.text(),
            "CREATE TABLE `t` (\n  `id` int\n) ENGINE=InnoDB;\n\
             \n\
             -- Triggers\n\
             CREATE TRIGGER `a` BEFORE INSERT ON `t` FOR EACH ROW SET NEW.id = 1;"
        );
        assert_eq!(ddl.source(), Source::Stored);
    }

    #[test]
    fn sqlite_text_keeps_its_statements_apart_by_a_blank_line() {
        let ddl = Ddl::new(
            Driver::Sqlite,
            "SQLite 3.46.0".into(),
            None,
            vec![
                part(PartKind::Object, &["CREATE TABLE t (id integer)"]),
                part(PartKind::Indexes, &["CREATE INDEX a ON t (id)"]),
                part(
                    PartKind::Triggers,
                    &["CREATE TRIGGER x AFTER UPDATE ON t BEGIN SELECT 1; END"],
                ),
            ],
        );
        assert_eq!(
            ddl.text(),
            "CREATE TABLE t (id integer);\n\
             \n\
             CREATE INDEX a ON t (id);\n\
             \n\
             CREATE TRIGGER x AFTER UPDATE ON t BEGIN SELECT 1; END;"
        );
        // With no comments, a part starts at its first statement.
        assert_eq!(
            ddl.laid_out().1,
            [
                (PartKind::Object, 0..1),
                (PartKind::Indexes, 2..3),
                (PartKind::Triggers, 4..5),
            ]
        );
    }

    /// A statement whose last line ends in a `--` comment gets its `;` on
    /// a line of its own: after the comment, it would be part of it.
    #[test]
    fn a_semicolon_is_never_put_in_a_comment() {
        assert_eq!(
            terminated("SET NEW.id = NEW.id -- keep it"),
            "SET NEW.id = NEW.id -- keep it\n;"
        );
        assert_eq!(terminated("SELECT 1 -- one;"), "SELECT 1 -- one;\n;");
        assert_eq!(terminated("SELECT 1;"), "SELECT 1;");
        assert_eq!(terminated("SELECT '--'"), "SELECT '--'\n;");
    }
}
