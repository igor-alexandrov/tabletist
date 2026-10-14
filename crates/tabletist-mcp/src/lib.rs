//! The MCP server through which agents use Tabletist's connections.
//!
//! An agent's tools only read. A refusal is a tool's result, with a code
//! that does not change and a sentence the agent can act on. Every call a
//! tool answers or refuses is written to the [`Log`]. A call that is no
//! tool's (an unknown name, an argument a tool does not take) is the
//! protocol's error, and no entry.
//!
//! A statement passes [`tabletist_db::agent::check_read`] before it reaches
//! a database, runs in a read-only transaction that is rolled back, and its
//! answer is kept under a size.
//!
//! The crate reaches a database only through a [`Gateway`], which lists the
//! connections an agent may name and opens one by its id. None of its types
//! holds where a database is or what opens it, so no tool can send that.
//!
//! Nothing here listens, and nothing in the app starts it yet: a
//! [`Server`] serves whatever transport it is handed.

mod answer;
mod gateway;
mod log;
mod protocol;
mod refusal;
mod server;
mod stop;
mod tools;

pub use answer::{MAX_ANSWER_BYTES, MAX_CELL_CHARS};
pub use gateway::{Failure, Gateway, Listed, Schemas, Session};
pub use log::{Entry, Log, LogError, Opened};
pub use refusal::{Refused, code};
pub use server::{Origin, Server, SessionInfo, Transport};
pub use tools::{Tool, all as tools};
