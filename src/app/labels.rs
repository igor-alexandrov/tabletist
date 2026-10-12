//! The reducer's part in the label of a table that foreign keys point at:
//! what the stored choice comes to for a workspace's table, and what
//! happens to a choice whose column is gone. The choice and the rule are
//! in `crate::labels`. Nothing else reads the store.

use tabletist_db::{ColumnInfo, Dialect, ObjectRef};

use super::App;
use crate::connections::ConnectionId;
use crate::labels::{self, LabelChoice, Labels, Resolved};
use crate::model::{ConnTabId, Fetch};

/// What labels the rows of `table`, whose columns the workspace of the
/// saved connection `conn` keeps as `kept`, in a picker whose foreign key
/// references `key`. None while the columns are not known. None too while
/// a chosen column is missing from columns that are being read again: it
/// is called gone only by columns that are fresh.
pub(super) fn resolved(
    labels: &Labels,
    (conn, dialect): (&ConnectionId, Dialect),
    kept: &Fetch<Vec<ColumnInfo>>,
    table: &ObjectRef,
    key: &str,
) -> Option<Resolved> {
    let columns = kept.value.as_deref()?;
    let choice = labels.choice(conn, table);
    let resolved = labels::resolve(&choice, dialect, columns, key);
    if resolved.gone.is_some() && kept.is_loading() {
        return None;
    }
    Some(resolved)
}

impl App {
    /// What labels the rows of `table` for the foreign keys of `tab` that
    /// reference its column `key`: the one resolver every view reads.
    pub fn label_of(&self, tab: ConnTabId, table: &ObjectRef, key: &str) -> Option<Resolved> {
        let workspace = self.workspace(tab)?;
        let of = (&workspace.conn_id, workspace.driver.dialect());
        resolved(&self.labels, of, workspace.columns.get(table)?, table, key)
    }

    /// The columns of `table` were just read. A column chosen as its label
    /// that is not among them is gone: the choice goes back to Automatic,
    /// and the column's name is kept to be said once.
    pub(super) fn check_label(&mut self, tab: ConnTabId, table: &ObjectRef) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        let fresh = workspace
            .columns
            .get(table)
            .filter(|kept| !kept.is_loading())
            .and_then(|kept| kept.value.as_deref());
        let Some(columns) = fresh else {
            return;
        };
        let conn = workspace.conn_id.clone();
        let LabelChoice::Column(name) = self.labels.choice(&conn, table) else {
            return;
        };
        if columns.iter().any(|column| column.name == name) {
            return;
        }
        self.labels.lost(&conn, table, &name);
        self.save_labels();
    }
}
