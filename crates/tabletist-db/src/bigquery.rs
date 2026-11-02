//! BigQuery, over its REST API: it has no wire protocol. A session is a
//! client and the project it bills. Everything is a query job; there are
//! no transactions, so the app opens every session read-only
//! ([`crate::Connection::connect_with`] forces it) and this adapter runs
//! only what [`crate::agent::check_read`] proves is a read: a script's
//! `INSERT` would really write here, where another engine's read-only
//! transaction would refuse it.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine as _;
use gcp_bigquery_client::Client;
use gcp_bigquery_client::error::BQError;
use gcp_bigquery_client::model::get_query_results_parameters::GetQueryResultsParameters;
use gcp_bigquery_client::model::query_parameter::QueryParameter;
use gcp_bigquery_client::model::query_parameter_type::QueryParameterType;
use gcp_bigquery_client::model::query_parameter_value::QueryParameterValue;
use gcp_bigquery_client::model::query_request::QueryRequest;
use gcp_bigquery_client::model::query_response::QueryResponse;
use gcp_bigquery_client::model::table_field_schema::TableFieldSchema;

use crate::adapter::Adapter;
use crate::agent::{Catalog, ColumnName, Limits, Param, Read, Sources};
use crate::{
    Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Ddl,
    Dialect, Error, Lookup, LookupPage, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result,
    RowPage, RowQuery, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag,
    Structure, Value, ValueKind, WriteOutcome,
};

/// How long a query job may run before the API answers without the rows
/// (the job runs on; its rows are then fetched).
const JOB_WAIT: Duration = Duration::from_secs(10);

/// A cancel handle's view of the session: the job running now, as its
/// (id, location), shared with the connection.
pub(crate) type Running = Arc<Mutex<Option<(String, Option<String>)>>>;

pub(crate) struct Conn {
    client: Client,
    /// The project queries are billed to and names resolve in.
    project: String,
    /// The job running now, as (id, location), so a cancel can reach it.
    running: Running,
}

/// What a [`CancelHandle`] needs to cancel the job a session runs.
/// Cloned handles share the one `running` slot.
#[derive(Clone)]
pub(crate) struct Cancel {
    client: Client,
    project: String,
    running: Running,
}

impl Cancel {
    /// Cancels the job running now; none running, nothing to do.
    pub(crate) async fn cancel(&self) -> Result<()> {
        let job = self.running.lock().expect("running-job lock").clone();
        let Some((job_id, location)) = job else {
            return Ok(());
        };
        self.client
            .job()
            .cancel_job(&self.project, &job_id, location.as_deref())
            .await
            .map_err(|error| Error::Connect(format!("could not cancel: {error}")))?;
        Ok(())
    }
}

/// `error` as the crate's: a cancelled job is [`Error::Cancelled`], a
/// transport failure a lost connection, anything else the server's words.
fn query_error(error: BQError) -> Error {
    let text = error.to_string();
    if matches!(error, BQError::RequestError(_)) {
        return Error::ConnectionLost(text);
    }
    if text.to_ascii_lowercase().contains("cancel") {
        return Error::Cancelled;
    }
    Error::query(text)
}

/// The field's type as the catalog names it: `INT64`, or `ARRAY<STRING>`
/// for a repeated one.
fn type_name(field: &TableFieldSchema) -> String {
    // FieldType serializes as the API's own spelling ("INT64").
    let base = serde_json::to_value(&field.r#type)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{:?}", field.r#type).to_ascii_uppercase());
    match field.mode.as_deref() {
        Some("REPEATED") => format!("ARRAY<{base}>"),
        _ => base,
    }
}

/// The columns a query's result schema describes.
fn column_metas(fields: &[TableFieldSchema]) -> Vec<ColumnMeta> {
    fields
        .iter()
        .map(|field| {
            let name = type_name(field);
            ColumnMeta {
                name: field.name.clone(),
                kind: ValueKind::from_bigquery_type(&name),
                type_name: name,
            }
        })
        .collect()
}

/// A TIMESTAMP cell, sent as seconds since the epoch (a float in a
/// string), as the app shows one: ISO, in UTC.
fn timestamp_text(text: &str) -> String {
    let Ok(seconds) = text.parse::<f64>() else {
        return text.to_owned();
    };
    let Some(at) = chrono::DateTime::from_timestamp_micros((seconds * 1e6) as i64) else {
        return text.to_owned();
    };
    at.format("%Y-%m-%d %H:%M:%S%.6f+00:00").to_string()
}

/// One cell, typed by its column's field. Everything but NULL arrives as
/// a string in the JSON; a RECORD or an ARRAY comes as nested JSON and is
/// shown as that text.
fn cell(field: &TableFieldSchema, value: &serde_json::Value) -> Value {
    let text = |value: &serde_json::Value| -> String {
        match value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        }
    };
    if value.is_null() {
        return Value::Null;
    }
    if field.mode.as_deref() == Some("REPEATED") {
        return Value::Text(text(value).into());
    }
    let shown = text(value);
    match type_name(field).as_str() {
        "INT64" | "INTEGER" => shown.parse().map(Value::Int).unwrap_or_else(|_| Value::Text(shown.into())),
        "FLOAT64" | "FLOAT" => shown
            .parse()
            .map(Value::Float)
            .unwrap_or_else(|_| Value::Text(shown.into())),
        "BOOL" | "BOOLEAN" => match shown.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => Value::Text(shown.into()),
        },
        "BYTES" => base64::engine::general_purpose::STANDARD
            .decode(&shown)
            .map(|bytes| Value::Bytes(bytes.into()))
            .unwrap_or_else(|_| Value::Text(shown.into())),
        "TIMESTAMP" => Value::Text(timestamp_text(&shown).into()),
        // NUMERIC and BIGNUMERIC stay text: exact, nothing rounded. A
        // DATETIME's `T` is how the app writes a moment's middle.
        "DATETIME" => Value::Text(shown.replace('T', " ").into()),
        _ => Value::Text(shown.into()),
    }
}

impl Conn {
    /// Connects: a client from the spec's service-account key file, or
    /// from Google Cloud's application default credentials where none is
    /// named. Probed with a one-row dataset listing so a bad key or
    /// project fails here and not on the first browse.
    pub async fn connect(spec: &ConnectSpec, _access: Access) -> Result<Self> {
        let project = spec.database.trim().to_owned();
        if project.is_empty() {
            return Err(Error::InvalidSpec("enter a GCP project".into()));
        }
        let client = match &spec.bigquery_key {
            Some(path) => {
                Client::from_service_account_key_file(&path.to_string_lossy()).await
            }
            None => Client::from_application_default_credentials().await,
        }
        .map_err(|error| Error::Auth(error.to_string()))?;
        let conn = Self {
            client,
            project,
            running: Arc::default(),
        };
        conn.client
            .dataset()
            .list(
                &conn.project,
                gcp_bigquery_client::dataset::ListOptions::default().max_results(1),
            )
            .await
            .map_err(|error| match error {
                BQError::AuthError(_) | BQError::YupAuthError(_) => Error::Auth(error.to_string()),
                other => Error::Connect(other.to_string()),
            })?;
        Ok(conn)
    }

    /// Runs `sql` as a query job and keeps at most `limit` rows. Waits
    /// [`JOB_WAIT`] for the job, then fetches rows of it as they come.
    /// The job's id is noted while it runs so [`CancelHandle`] reaches it.
    async fn query(
        &self,
        sql: &str,
        limit: usize,
        params: Vec<QueryParameter>,
    ) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>, Duration)> {
        let started = Instant::now();
        let mut request = QueryRequest::new(sql);
        request.max_results = Some(limit as i32);
        request.timeout_ms = Some(JOB_WAIT.as_millis() as i32);
        if !params.is_empty() {
            request.parameter_mode = Some("POSITIONAL".to_owned());
            request.query_parameters = Some(params);
        }
        let response = self
            .client
            .job()
            .query(&self.project, request)
            .await
            .map_err(query_error)?;
        // A job that did not finish within the wait has its rows fetched
        // by its reference; both kinds page the same way.
        let job = response.job_reference.clone();
        if let Some(reference) = &job {
            let mut running = self.running.lock().expect("running-job lock");
            *running = reference
                .job_id
                .clone()
                .map(|id| (id, reference.location.clone()));
        }
        let result = self.collect(response, job, limit).await;
        *self.running.lock().expect("running-job lock") = None;
        result.map(|(columns, rows)| (columns, rows, started.elapsed()))
    }

    /// The rows of a finished or running query's first response, and of
    /// its later pages, at most `limit` of them.
    async fn collect(
        &self,
        mut response: QueryResponse,
        job: Option<gcp_bigquery_client::model::job_reference::JobReference>,
        limit: usize,
    ) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>)> {
        let mut columns = Vec::new();
        let mut rows: Vec<Vec<Value>> = Vec::new();
        // A job still running when the wait ran out answers with neither
        // rows nor a page token: its results are asked for by the job's
        // id, page by page once it completes.
        loop {
            let complete = response.job_complete.unwrap_or(true);
            if let Some(schema) = response.schema.as_ref()
                && let Some(fields) = schema.fields.as_ref()
            {
                columns = column_metas(fields);
                if let Some(page) = response.rows.take() {
                    for row in page {
                        let cells = row.columns.unwrap_or_default();
                        rows.push(
                            fields
                                .iter()
                                .zip(cells.iter())
                                .map(|(field, c)| {
                                    cell(field, c.value.as_ref().unwrap_or(&serde_json::Value::Null))
                                })
                                .collect(),
                        );
                    }
                }
            }
            if rows.len() >= limit {
                rows.truncate(limit);
                return Ok((columns, rows));
            }
            let token = response.page_token.take();
            if complete && token.is_none() {
                return Ok((columns, rows));
            }
            let Some(job_id) = job.as_ref().and_then(|job| job.job_id.clone()) else {
                return Ok((columns, rows));
            };
            let page = GetQueryResultsParameters {
                page_token: token,
                max_results: Some((limit - rows.len()) as i32),
                location: job.as_ref().and_then(|job| job.location.clone()),
                ..Default::default()
            };
            response = self
                .client
                .job()
                .get_query_results(&self.project, &job_id, page)
                .await
                .map_err(query_error)?
                .into();
        }
    }

    /// The table's fields, or an error that there is no such table.
    async fn fields(&self, object: &ObjectRef) -> Result<Vec<TableFieldSchema>> {
        let table = self
            .client
            .table()
            .get(&self.project, &object.schema, &object.name, None)
            .await
            .map_err(query_error)?;
        Ok(table.schema.fields.unwrap_or_default())
    }

    /// The names of the table's BYTES columns, for a filter that compares
    /// a value that reads as bytes: see [`Dialect::select_rows`].
    async fn binary_columns(&self, query: &RowQuery) -> Result<Vec<String>> {
        if !crate::dialect::reads_bytes(query) {
            return Ok(Vec::new());
        }
        Ok(self
            .fields(&query.object)
            .await?
            .iter()
            .filter(|field| type_name(field) == "BYTES")
            .map(|field| field.name.clone())
            .collect())
    }
}

impl Adapter for Conn {
    /// Always: the API is HTTPS-only.
    fn is_encrypted(&self) -> bool {
        true
    }

    fn cancel_handle(&self) -> CancelHandle {
        CancelHandle(CancelInner::BigQuery(Box::new(Cancel {
            client: self.client.clone(),
            project: self.project.clone(),
            running: Arc::clone(&self.running),
        })))
    }

    /// BigQuery the service has no version to name.
    async fn server_version(&self) -> Result<String> {
        Ok("BigQuery".to_owned())
    }

    /// A TIMESTAMP comes with its epoch, a DATETIME is zone-less.
    async fn session_zone(&self) -> Result<Option<String>> {
        Ok(None)
    }

    /// Projects are not listed (that is the Resource Manager's, not
    /// BigQuery's), and there is no switching to one: a session's project
    /// is fixed.
    async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// The project's datasets, as schemas.
    async fn list_schemas(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        let mut token = None;
        loop {
            let mut options = gcp_bigquery_client::dataset::ListOptions::default();
            if let Some(token) = token {
                options = options.page_token(token);
            }
            let page = self
                .client
                .dataset()
                .list(&self.project, options)
                .await
                .map_err(query_error)?;
            names.extend(
                page.datasets
                    .into_iter()
                    .map(|dataset| dataset.dataset_reference.dataset_id),
            );
            if names.len() >= MAX_LISTED as usize {
                names.truncate(MAX_LISTED as usize);
                return Ok(names);
            }
            match page.next_page_token {
                Some(next) => token = Some(next),
                None => return Ok(names),
            }
        }
    }

    /// The dataset's tables and views, as objects.
    async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let mut objects = Vec::new();
        let mut token = None;
        loop {
            let mut options = gcp_bigquery_client::table::ListOptions::default();
            if let Some(token) = token {
                options = options.page_token(token);
            }
            let page = self
                .client
                .table()
                .list(&self.project, schema, options)
                .await
                .map_err(query_error)?;
            objects.extend(page.tables.unwrap_or_default().into_iter().map(|table| {
                ObjectInfo {
                    name: table.table_reference.table_id,
                    kind: match table.r#type.as_deref() {
                        Some("VIEW") => ObjectKind::View,
                        Some("MATERIALIZED_VIEW") => ObjectKind::MaterializedView,
                        _ => ObjectKind::Table,
                    },
                    estimated_rows: None,
                }
            }));
            if objects.len() >= MAX_LISTED as usize {
                objects.truncate(MAX_LISTED as usize);
                return Ok(objects);
            }
            match page.next_page_token {
                Some(next) => token = Some(next),
                None => return Ok(objects),
            }
        }
    }

    /// The table's schema as its structure. BigQuery has no indexes; its
    /// key constraints are unenforced and name no row, so none are read.
    async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let table = self
            .client
            .table()
            .get(&self.project, &object.schema, &object.name, None)
            .await
            .map_err(query_error)?;
        let columns = table
            .schema
            .fields
            .unwrap_or_default()
            .iter()
            .map(|field| ColumnInfo {
                name: field.name.clone(),
                type_name: type_name(field),
                nullable: field.mode.as_deref() != Some("REQUIRED"),
                default: None,
                comment: field.description.clone().filter(|text| !text.is_empty()),
                allowed_values: None,
                generated: false,
                identity: false,
            })
            .collect();
        Ok(Structure {
            columns,
            comment: table.description.filter(|text| !text.is_empty()),
            ..Structure::default()
        })
    }

    async fn ddl(&self, _object: &ObjectRef) -> Result<Ddl> {
        Err(Error::Unsupported("BigQuery DDL"))
    }

    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::BigQuery.select_rows(query, &[], &binary);
        debug_assert!(sql.params.is_empty());
        let limit = query.limit as usize;
        let (columns, mut rows, elapsed) = self.query(&sql.text, limit + 1, Vec::new()).await?;
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        Ok(RowPage {
            columns,
            rows,
            has_more,
            ordered_by_key: false,
            elapsed,
        })
    }

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::BigQuery.count_rows(query, &binary);
        let (_, rows, _) = self.query(&sql.text, 1, Vec::new()).await?;
        rows.first()
            .and_then(|row| row.first())
            .and_then(|value| match value {
                Value::Int(count) => Some(*count as u64),
                Value::Text(text) => text.parse().ok(),
                _ => None,
            })
            .ok_or_else(|| Error::query("the count returned no number"))
    }

    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        let sql = Dialect::BigQuery.lookup(lookup);
        let (_, rows, _) = self.query(&sql.text, lookup.limit as usize + 1, Vec::new()).await?;
        crate::lookup::page(rows, lookup)
    }

    /// Each statement runs as its own query job; only a statement
    /// [`crate::agent::check_read`] proves a read is sent, since BigQuery
    /// has no read-only transaction to fence the rest.
    async fn run_script(
        &self,
        texts: Vec<String>,
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        if mode == ScriptMode::Write {
            return Err(Error::ReadOnly);
        }
        let mut outcome = ScriptOutcome::default();
        for text in texts {
            if stop.is_stopped() {
                outcome.stopped = true;
                outcome.results.push(StatementResult {
                    elapsed: Duration::ZERO,
                    outcome: StatementOutcome::Cancelled,
                });
                continue;
            }
            let started = Instant::now();
            let run = async {
                let read = crate::agent::check_read(Dialect::BigQuery, &text, &[])
                    .map_err(Error::Agent)?;
                let (columns, rows, _) =
                    self.query(read.text(), limit as usize + 1, Vec::new()).await?;
                Ok((columns, rows))
            };
            match run.await {
                Ok((columns, mut rows)) => {
                    let truncated = rows.len() > limit as usize;
                    rows.truncate(limit as usize);
                    outcome.results.push(StatementResult {
                        elapsed: started.elapsed(),
                        outcome: StatementOutcome::Rows {
                            columns,
                            rows,
                            truncated,
                        },
                    });
                }
                Err(error) => {
                    let failed = crate::script::statement_failed(error, None)?;
                    outcome.results.push(StatementResult {
                        elapsed: started.elapsed(),
                        outcome: failed,
                    });
                    break;
                }
            }
        }
        Ok(outcome)
    }

    /// The checked statement runs as a read-only script of one does, its
    /// values bound by the API as typed parameters. BigQuery keeps no
    /// maskable columns here: `agents_catalog` is unsupported, so a
    /// connection with masks never reaches this.
    async fn run_read(
        &self,
        read: &Read,
        limits: Limits,
        _sources: &Sources,
        stop: &StopFlag,
    ) -> Result<(ScriptOutcome, Vec<bool>)> {
        let params = read
            .params()
            .iter()
            .map(|param| {
                let (kind, value) = match param {
                    Param::Null => ("STRING", None),
                    Param::Bool(flag) => ("BOOL", Some(flag.to_string())),
                    Param::Int(number) => ("INT64", Some(number.to_string())),
                    Param::Float(number) => ("FLOAT64", Some(number.to_string())),
                    Param::Text(text) => ("STRING", Some(text.clone())),
                };
                QueryParameter {
                    name: None,
                    parameter_type: Some(QueryParameterType {
                        r#type: kind.to_owned(),
                        ..Default::default()
                    }),
                    parameter_value: Some(QueryParameterValue {
                        value,
                        ..Default::default()
                    }),
                }
            })
            .collect();
        let mut outcome = ScriptOutcome::default();
        if stop.is_stopped() {
            outcome.stopped = true;
            outcome.results.push(StatementResult {
                elapsed: Duration::ZERO,
                outcome: StatementOutcome::Cancelled,
            });
            return Ok((outcome, Vec::new()));
        }
        let started = Instant::now();
        match self
            .query(read.text(), limits.rows as usize + 1, params)
            .await
        {
            Ok((columns, mut rows, _)) => {
                let truncated = rows.len() > limits.rows as usize;
                rows.truncate(limits.rows as usize);
                outcome.results.push(StatementResult {
                    elapsed: started.elapsed(),
                    outcome: StatementOutcome::Rows {
                        columns,
                        rows,
                        truncated,
                    },
                });
            }
            Err(error) => {
                let failed = crate::script::statement_failed(error, None)?;
                outcome.results.push(StatementResult {
                    elapsed: started.elapsed(),
                    outcome: failed,
                });
            }
        }
        Ok((outcome, Vec::new()))
    }

    /// Nothing of BigQuery's catalog is read for an agent's rules, so a
    /// connection that hides or masks anything fails closed here.
    async fn agents_catalog(&self, _masked: &[ColumnName]) -> Result<Catalog> {
        Err(Error::Unsupported(
            "hiding or masking columns on a BigQuery connection",
        ))
    }

    /// BigQuery has no row locks to save rows with; a session opens
    /// read-only, so this is never reached through [`crate::Connection`].
    async fn write(&self, _changes: &ChangeSet, _stop: &StopFlag) -> Result<WriteOutcome> {
        Err(Error::Unsupported("editing rows in BigQuery"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gcp_bigquery_client::model::field_type::FieldType;

    fn field(kind: FieldType, mode: Option<&str>) -> TableFieldSchema {
        TableFieldSchema {
            mode: mode.map(str::to_owned),
            ..TableFieldSchema::new("c", kind)
        }
    }

    fn json(text: &str) -> serde_json::Value {
        serde_json::Value::String(text.to_owned())
    }

    #[test]
    fn cells_read_by_their_fields_type() {
        assert_eq!(
            cell(&field(FieldType::Int64, None), &json("42")),
            Value::Int(42)
        );
        // An integer too wide stays text; nothing is rounded.
        let wide = json("9223372036854775808");
        assert!(matches!(
            cell(&field(FieldType::Int64, None), &wide),
            Value::Text(_)
        ));
        assert_eq!(
            cell(&field(FieldType::Float64, None), &json("1.5")),
            Value::Float(1.5)
        );
        assert_eq!(
            cell(&field(FieldType::Boolean, None), &json("true")),
            Value::Bool(true)
        );
        // NUMERIC stays exact text.
        assert_eq!(
            cell(&field(FieldType::Numeric, None), &json("0.1")),
            Value::Text("0.1".into())
        );
        assert_eq!(
            cell(&field(FieldType::Bytes, None), &json("AAEC")),
            Value::Bytes(Box::new([0, 1, 2]))
        );
        assert_eq!(
            cell(&field(FieldType::String, None), &serde_json::Value::Null),
            Value::Null
        );
        // A TIMESTAMP's epoch seconds read as a moment, in UTC.
        assert_eq!(
            cell(&field(FieldType::Timestamp, None), &json("0.0")),
            Value::Text("1970-01-01 00:00:00.000000+00:00".into())
        );
        // A DATETIME's T is the app's own middle of a moment.
        assert_eq!(
            cell(&field(FieldType::Datetime, None), &json("2024-02-03T04:05:06")),
            Value::Text("2024-02-03 04:05:06".into())
        );
        // Repeated and nested values show as the JSON the API gives.
        let array = serde_json::json!(["a", "b"]);
        assert_eq!(
            cell(&field(FieldType::String, Some("REPEATED")), &array),
            Value::Text(r#"["a","b"]"#.into())
        );
        assert_eq!(type_name(&field(FieldType::String, Some("REPEATED"))), "ARRAY<STRING>");
        assert_eq!(type_name(&field(FieldType::Int64, None)), "INT64");
    }

    #[test]
    fn a_script_only_runs_what_proves_a_read() {
        // The guard the adapter's run_script stands on.
        assert!(crate::agent::check_read(crate::Dialect::BigQuery, "SELECT 1", &[]).is_ok());
        assert!(
            crate::agent::check_read(
                crate::Dialect::BigQuery,
                "WITH t AS (SELECT 1) SELECT * FROM t",
                &[]
            )
            .is_ok()
        );
        for text in [
            "INSERT INTO d.t VALUES (1)",
            "CREATE TABLE d.t (x INT64)",
            "EXECUTE IMMEDIATE 'SELECT 1'",
            "BEGIN TRANSACTION",
            "SELECT 1; SELECT 2",
        ] {
            assert!(
                crate::agent::check_read(crate::Dialect::BigQuery, text, &[]).is_err(),
                "{text} must be refused"
            );
        }
    }
}
