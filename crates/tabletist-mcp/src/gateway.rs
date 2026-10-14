//! How the server reaches a database: through whoever holds the
//! connections, which lists those an agent may name and opens one by its
//! id. Nothing here says where a database is or what opens it, so no tool
//! can send that: the server never has it.

use std::future::Future;

use tabletist_db::agent::{Answer, Limits, Read};
use tabletist_db::{Dialect, ObjectInfo, ObjectRef, Structure};
use tokio_util::sync::CancellationToken;

/// A connection as an agent may know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// The saved connection's id. The log keeps it, so that a connection
    /// keeps its entries when it is renamed. It is never sent to an agent.
    pub id: String,
    /// What an agent calls the connection. No two listed connections share
    /// a name.
    pub name: String,
    pub dialect: Dialect,
    /// The most rows of an answer, and how long a call may take.
    pub limits: Limits,
}

/// The schemas of a connection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schemas {
    /// Every schema the session lists, the databases' own among them.
    pub names: Vec<String>,
    /// The one a name without a schema is looked up in: the schema the app
    /// opens first for this connection. `None` when there is no saying.
    pub home: Option<String>,
}

/// Why the gateway did not answer.
#[derive(Debug)]
pub enum Failure {
    /// The connection cannot be opened without a person, or could not be
    /// opened at all. Why is for the person: it can name a host or a user.
    Unavailable,
    /// Another call held the connection for as long as this one may wait.
    Busy,
    /// The database's answer, or the driver's.
    Database(tabletist_db::Error),
}

impl From<tabletist_db::Error> for Failure {
    fn from(error: tabletist_db::Error) -> Self {
        Self::Database(error)
    }
}

/// Whoever holds the connections agents may use.
///
/// A connection whose access is Off is not among [`Gateway::connections`]
/// and cannot be opened: to the server it does not exist.
pub trait Gateway: Send + Sync + 'static {
    type Session: Session;

    /// The connections agents may use, as they are now.
    fn connections(&self) -> Vec<Listed>;

    /// The session of the connection with this id, for one call. A
    /// connection runs one call at a time: a second one waits here, and is
    /// [`Failure::Busy`] when it has waited the connection's timeout.
    /// Opening is the implementation's to bound as well: the server's own
    /// timer starts when it has the session.
    ///
    /// Once `stop` is cancelled, whatever the session is doing is ended,
    /// and the call that asked answers [`tabletist_db::Error::Cancelled`].
    /// A statement that was sent is awaited to its end all the same: the
    /// futures of a session are never dropped half way.
    fn open(
        &self,
        id: &str,
        stop: &CancellationToken,
    ) -> impl Future<Output = Result<Self::Session, Failure>> + Send;
}

/// A connection's session, held for one call. It is read-only at the
/// database, whatever the connection's own setting is.
pub trait Session: Send + Sync {
    fn schemas(&self) -> impl Future<Output = Result<Schemas, Failure>> + Send;

    /// The tables and views of a schema, by name.
    fn objects(
        &self,
        schema: &str,
    ) -> impl Future<Output = Result<Vec<ObjectInfo>, Failure>> + Send;

    fn describe(
        &self,
        object: &ObjectRef,
    ) -> impl Future<Output = Result<Structure, Failure>> + Send;

    /// Runs a statement that passed the check
    /// ([`tabletist_db::Connection::run_agent_read`]).
    fn read(
        &self,
        read: &Read,
        limits: Limits,
    ) -> impl Future<Output = Result<Answer, Failure>> + Send;
}
