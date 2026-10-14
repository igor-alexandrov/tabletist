//! The MCP server through which agents use Tabletist's connections.
//!
//! So far it is the [`Log`] of calls, and nothing that makes a call.

mod log;

pub use log::{Entry, Log, LogError, Opened};
