//! What the adapter of every database provides.

use crate::{
    CancelHandle, ChangeSet, ObjectInfo, ObjectRef, Result, RowPage, RowQuery, ScriptMode,
    ScriptOutcome, StopFlag, Structure, WriteOutcome,
};

/// An open session with one kind of database. [`crate::Connection`] holds
/// one and forwards to it, and says there what each of these does.
///
/// No method has a default. An adapter answers every one, if only to say
/// that it has nothing to list: a default would answer for a new database
/// without anyone choosing it.
pub(crate) trait Adapter {
    fn is_encrypted(&self) -> bool;

    fn cancel_handle(&self) -> CancelHandle;

    async fn server_version(&self) -> Result<String>;

    async fn list_databases(&self) -> Result<Vec<String>>;

    async fn list_schemas(&self) -> Result<Vec<String>>;

    async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>>;

    async fn describe(&self, object: &ObjectRef) -> Result<Structure>;

    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage>;

    async fn count_rows(&self, query: &RowQuery) -> Result<u64>;

    /// `texts` are statements [`crate::sql::refusal`] has let through.
    async fn run_script(
        &self,
        texts: Vec<String>,
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome>;

    /// `changes` have passed their own check, and the session may write.
    async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome>;
}
