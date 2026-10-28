//! The app's backend, answered by a real SQLite session reading the
//! scenario's file. Every SQLite session is `query_only`: nothing is
//! written, saved or stored. The query history is kept in memory.
//!
//! A save is answered only where the scenario says what someone else
//! changed meanwhile, by [`Server`]: with the rows the change touched,
//! changed on the server. The file itself is never written.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use tabletist_db::{Access, ConnectSpec, Connection, Secrets, StopFlag};

use super::server::Server;

use crate::backend::{Command, Event, HistoryCommand, HistoryEvent};
use crate::history::Entry;
use crate::model::Action;
use crate::testing::Harness;

/// The file's connection, how many of the app's commands it answered, the
/// statements the history was given, oldest first, and the server a save
/// is answered from.
pub struct Live {
    runtime: tokio::runtime::Runtime,
    connection: Connection,
    answered: usize,
    history: Vec<Entry>,
    /// The copy of the file with what changed meanwhile: None where the
    /// scenario says nothing changed, and no save is answered.
    server: Option<Server>,
}

impl Live {
    /// The SQLite file at `path`, opened read-only, or why it would not
    /// open.
    pub fn open(path: &Path) -> tabletist_db::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let spec = ConnectSpec::sqlite(path);
        let connection = runtime.block_on(Connection::connect(&spec, &Secrets::default()))?;
        Ok(Self {
            runtime,
            connection,
            answered: 0,
            history: Vec::new(),
            server: None,
        })
    }

    /// Answers each save from a copy of the file at `path`, written to
    /// `copy`, with `sql` run on it: see [`Server::stage`].
    pub fn meanwhile(&mut self, path: &Path, copy: &Path, sql: &str) -> Result<(), String> {
        self.server = Some(Server::stage(&self.runtime, path, copy, sql)?);
        Ok(())
    }

    pub fn access(&self) -> Access {
        self.connection.access()
    }

    /// Answers every command the app sent since the last call, and those
    /// the answers make it send. A save that the server would not answer
    /// with a conflict stops it: what it would answer, the runner cannot.
    pub fn pump(&mut self, harness: &mut Harness) -> Result<(), String> {
        while let Some(command) = harness.app.backend.sent.get(self.answered) {
            self.answered += 1;
            let connection = &self.connection;
            let run = &self.runtime;
            let history = &mut self.history;
            let server = &self.server;
            let event = match command {
                Command::Connect {
                    session,
                    request,
                    access,
                    ..
                } => Some(Event::Connected {
                    session: *session,
                    request: *request,
                    driver: connection.driver(),
                    encrypted: false,
                    access: *access,
                }),
                Command::ListSchemas { session, request } => Some(Event::Schemas {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.list_schemas()),
                }),
                Command::ListObjects {
                    session,
                    request,
                    schema,
                } => Some(Event::Objects {
                    session: *session,
                    request: *request,
                    schema: schema.clone(),
                    result: run.block_on(connection.list_objects(schema)),
                }),
                Command::Describe {
                    session,
                    request,
                    object,
                } => Some(Event::Structure {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.describe(object)),
                }),
                Command::FetchRows {
                    session,
                    request,
                    query,
                } => Some(Event::Rows {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.fetch_rows(query)),
                }),
                Command::CountRows {
                    session,
                    request,
                    query,
                } => Some(Event::Count {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.count_rows(query)),
                }),
                Command::Lookup {
                    session,
                    request,
                    lookup,
                } => Some(Event::Lookup {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.lookup(lookup)),
                }),
                Command::ReadDdl {
                    session,
                    request,
                    object,
                } => Some(Event::Ddl {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.ddl(object)),
                }),
                // The zone database is read here, as the backend does.
                Command::SessionZone { session, request } => Some(Event::SessionZone {
                    session: *session,
                    request: *request,
                    result: run
                        .block_on(connection.session_zone())
                        .map(|named| named.as_deref().and_then(crate::zones::session)),
                }),
                Command::ListDatabases { session, request } => Some(Event::Databases {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.list_databases()),
                }),
                Command::ServerVersion { session, request } => Some(Event::ServerVersion {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.server_version()),
                }),
                Command::RunSql {
                    session,
                    request,
                    statements,
                    limit,
                    mode,
                    ..
                } => Some(Event::SqlRan {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.run_script(
                        statements,
                        *limit,
                        *mode,
                        &StopFlag::new(),
                    )),
                    cancel: None,
                }),
                Command::Write {
                    session,
                    request,
                    changes,
                } => match server {
                    None => None,
                    Some(server) => Some(Event::Written {
                        session: *session,
                        request: *request,
                        result: Ok(server.answer(run, changes)?),
                    }),
                },
                Command::History(HistoryCommand::Append { entry, .. }) => {
                    history.push(entry.as_ref().clone());
                    None
                }
                Command::History(HistoryCommand::Load {
                    connection,
                    keep,
                    asked,
                    ..
                }) => Some(Event::History(HistoryEvent::Loaded {
                    asked: *asked,
                    result: Ok(super::history::newest(history, connection, *keep)),
                })),
                // Nothing is written, saved, stored, watched or closed here.
                Command::Test { .. }
                | Command::Close { .. }
                | Command::Cancel { .. }
                | Command::LoadSecret { .. }
                | Command::StoreSecret { .. }
                | Command::Save { .. }
                | Command::WatchSettings { .. }
                | Command::LoadZones
                | Command::EditSettingsFile { .. }
                | Command::RevealSettingsFile { .. }
                | Command::OpenKeysList { .. }
                | Command::ReadTheme { .. }
                | Command::ReadFont { .. }
                | Command::ListFonts
                | Command::Flush { .. }
                | Command::Agents(_)
                | Command::Updates(_)
                | Command::History(
                    HistoryCommand::Clear { .. }
                    | HistoryCommand::Trim { .. }
                    | HistoryCommand::Held { .. },
                ) => None,
            };
            if let Some(event) = event {
                harness.app.apply(Action::Backend(event));
            }
        }
        Ok(())
    }
}
