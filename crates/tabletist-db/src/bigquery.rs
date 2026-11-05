//! BigQuery, over its REST API: it has no wire protocol. A session is a
//! client and the project it bills. Everything is a query job; there are
//! no transactions, so the app opens every session read-only
//! ([`crate::Connection::connect_with`] forces it) and this adapter runs
//! only what [`crate::agent::check_read`] proves is a read: a script's
//! `INSERT` would really write here, where another engine's read-only
//! transaction would refuse it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use gcp_bigquery_client::Client;
use gcp_bigquery_client::error::BQError;
use gcp_bigquery_client::model::get_query_results_parameters::GetQueryResultsParameters;
use gcp_bigquery_client::model::job::Job;
use gcp_bigquery_client::model::job_configuration::JobConfiguration;
use gcp_bigquery_client::model::job_configuration_query::JobConfigurationQuery;
use gcp_bigquery_client::model::job_reference::JobReference;
use gcp_bigquery_client::model::query_parameter::QueryParameter;
use gcp_bigquery_client::model::query_parameter_type::QueryParameterType;
use gcp_bigquery_client::model::query_parameter_value::QueryParameterValue;
use gcp_bigquery_client::model::table_field_schema::TableFieldSchema;
use gcp_bigquery_client::model::table_row::TableRow;

use crate::adapter::Adapter;
use crate::agent::{Catalog, ColumnName, Limits, Param, Read, Sources};
use crate::{
    Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, ConnectSpec,
    Ddl, Dialect, Error, FilterOp, Lookup, LookupPage, MAX_LISTED, ObjectInfo, ObjectKind,
    ObjectRef, Result, RowPage, RowQuery, ScriptMode, ScriptOutcome, StatementOutcome,
    StatementResult, StopFlag, Structure, Value, ValueKind, WriteOutcome,
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

/// A unique id of this process's own: job ids are letters, digits,
/// dashes and underscores. The run's start is in it: a pid the system
/// hands out again would repeat an earlier process's ids without it.
fn next_job_id() -> String {
    static RUN: LazyLock<u128> = LazyLock::new(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    });
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "tabletist-{}-{:x}-{}",
        std::process::id(),
        *RUN,
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
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
        "INT64" | "INTEGER" => shown
            .parse()
            .map(Value::Int)
            .unwrap_or_else(|_| Value::Text(shown.into())),
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

/// A result page's rows as values, typed by the schema's `fields`.
fn rows_of(fields: &[TableFieldSchema], page: Vec<TableRow>) -> Vec<Vec<Value>> {
    page.into_iter()
        .map(|row| {
            let cells = row.columns.unwrap_or_default();
            fields
                .iter()
                .zip(cells.iter())
                .map(|(field, c)| cell(field, c.value.as_ref().unwrap_or(&serde_json::Value::Null)))
                .collect()
        })
        .collect()
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
            Some(path) => Client::from_service_account_key_file(&path.to_string_lossy()).await,
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
                // The API was reached and answered: a bad project or a
                // missing permission, not a network failure.
                BQError::ResponseError { .. } => Error::query(error.to_string()),
                other => Error::Connect(other.to_string()),
            })?;
        Ok(conn)
    }

    /// Runs `sql` as a query job and keeps at most `limit` rows. The job
    /// gets its id here, before anything is sent, and is noted in
    /// `running` first: a cancel reaches it from its first moment, while
    /// the job still runs. `timeout` ends the job on the server
    /// (an agent's statement timeout; browsing has none).
    ///
    /// Every statement goes through [`crate::agent::check_read`], here in
    /// the one place: a filter's raw WHERE is written into the built SQL,
    /// and BigQuery would run a second statement it smuggles.
    async fn query(
        &self,
        sql: &str,
        params: &[Param],
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>, Duration)> {
        let read =
            crate::agent::check_read(Dialect::BigQuery, sql, params).map_err(Error::Agent)?;
        let started = Instant::now();
        let job_id = next_job_id();
        *self.running.lock().expect("running-job lock") = Some((job_id.clone(), None));
        let result = self
            .run_job(&read, limit, timeout, job_id)
            .await
            .map(|(columns, rows)| (columns, rows, started.elapsed()));
        *self.running.lock().expect("running-job lock") = None;
        result
    }

    /// Inserts a job of `query` (ended by `timeout` on the server where
    /// one is given) and returns its reference, noting where the server
    /// runs it in `running` so a cancel can reach it there.
    async fn insert_job(
        &self,
        job_id: String,
        query: JobConfigurationQuery,
        timeout: Option<Duration>,
    ) -> Result<JobReference> {
        let job = Job {
            job_reference: Some(JobReference {
                job_id: Some(job_id),
                project_id: Some(self.project.clone()),
                location: None,
            }),
            configuration: Some(JobConfiguration {
                query: Some(query),
                job_timeout_ms: timeout.map(|timeout| timeout.as_millis().to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let inserted = self
            .client
            .job()
            .insert(&self.project, job)
            .await
            .map_err(query_error)?;
        // The server says where the job runs, which its pages are read by.
        let reference = inserted
            .job_reference
            .expect("a job answers with its reference");
        if let Ok(mut running) = self.running.lock()
            && let Some(current) = running.as_mut()
            && current.0 == reference.job_id.clone().unwrap_or_default()
        {
            current.1 = reference.location.clone();
        }
        Ok(reference)
    }

    /// The job of a checked read, from its insert to its last page.
    async fn run_job(
        &self,
        read: &Read,
        limit: usize,
        timeout: Option<Duration>,
        job_id: String,
    ) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>)> {
        let clamped = i32::try_from(limit).unwrap_or(i32::MAX);
        let params: Vec<QueryParameter> = read
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
        let reference = self
            .insert_job(
                job_id,
                JobConfigurationQuery {
                    query: read.text().to_owned(),
                    use_legacy_sql: Some(false),
                    parameter_mode: (!params.is_empty()).then(|| "POSITIONAL".to_owned()),
                    query_parameters: (!params.is_empty()).then_some(params),
                    ..Default::default()
                },
                timeout,
            )
            .await?;
        let mut columns = Vec::new();
        let mut rows: Vec<Vec<Value>> = Vec::new();
        let mut token = None;
        loop {
            let page = GetQueryResultsParameters {
                page_token: token.clone(),
                max_results: Some(clamped - i32::try_from(rows.len()).unwrap_or(clamped)),
                location: reference.location.clone(),
                timeout_ms: Some(JOB_WAIT.as_millis() as i32),
                ..Default::default()
            };
            let job_id = reference.job_id.clone().expect("noted above");
            let mut response: gcp_bigquery_client::model::query_response::QueryResponse = self
                .client
                .job()
                .get_query_results(&self.project, &job_id, page)
                .await
                .map_err(query_error)?
                .into();
            let complete = response.job_complete.unwrap_or(true);
            if let Some(schema) = response.schema.as_ref()
                && let Some(fields) = schema.fields.as_ref()
            {
                columns = column_metas(fields);
                if let Some(page) = response.rows.take() {
                    rows.extend(rows_of(fields, page));
                }
            }
            if rows.len() >= limit {
                rows.truncate(limit);
                return Ok((columns, rows));
            }
            let next = response.page_token.take();
            // A token that does not advance would read the page again
            // forever.
            if next.is_some() && next == token {
                return Err(Error::query("BigQuery returned the same page token twice"));
            }
            token = next;
            if complete && token.is_none() {
                return Ok((columns, rows));
            }
        }
    }

    /// Streams `sql` as an export's job, once [`crate::agent::check_read`]
    /// proves it a read: BigQuery has no read-only transaction to fence
    /// anything else.
    async fn stream_read(
        &self,
        sql: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let read = crate::agent::check_read(Dialect::BigQuery, sql, &[]).map_err(Error::Agent)?;
        let job_id = next_job_id();
        *self.running.lock().expect("running-job lock") = Some((job_id.clone(), None));
        let result = self.stream_job(read.text(), batch, to, stop, job_id).await;
        *self.running.lock().expect("running-job lock") = None;
        result
    }

    /// The job of an export's checked read, a page at a time: the columns
    /// once, then batches of up to `batch` rows, so no more than a page is
    /// held at once. There are no parameters: an export's statement has
    /// its values written in. Returns the rows sent.
    async fn stream_job(
        &self,
        sql: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
        job_id: String,
    ) -> Result<u64> {
        let page_size = i32::try_from(batch).unwrap_or(i32::MAX);
        let reference = self
            .insert_job(
                job_id,
                JobConfigurationQuery {
                    query: sql.to_owned(),
                    use_legacy_sql: Some(false),
                    ..Default::default()
                },
                None,
            )
            .await?;
        let mut columns_sent = false;
        let mut sent = 0u64;
        let mut token = None;
        loop {
            if stop.is_stopped() {
                return Err(Error::Cancelled);
            }
            let page = GetQueryResultsParameters {
                page_token: token.clone(),
                max_results: Some(page_size),
                location: reference.location.clone(),
                timeout_ms: Some(JOB_WAIT.as_millis() as i32),
                ..Default::default()
            };
            let job_id = reference.job_id.clone().expect("noted above");
            let mut response: gcp_bigquery_client::model::query_response::QueryResponse = self
                .client
                .job()
                .get_query_results(&self.project, &job_id, page)
                .await
                .map_err(query_error)?
                .into();
            let complete = response.job_complete.unwrap_or(true);
            if let Some(schema) = response.schema.as_ref()
                && let Some(fields) = schema.fields.as_ref()
            {
                if !columns_sent {
                    crate::stream::send(to, crate::Streamed::Columns(column_metas(fields))).await?;
                    columns_sent = true;
                }
                if let Some(page) = response.rows.take() {
                    let mut values: Vec<Vec<Value>> = Vec::with_capacity(batch);
                    for row in rows_of(fields, page) {
                        values.push(row);
                        if values.len() == batch {
                            sent += values.len() as u64;
                            crate::stream::send(
                                to,
                                crate::Streamed::Rows(std::mem::take(&mut values)),
                            )
                            .await?;
                        }
                    }
                    if !values.is_empty() {
                        sent += values.len() as u64;
                        crate::stream::send(to, crate::Streamed::Rows(values)).await?;
                    }
                }
            }
            let next = response.page_token.take();
            // A token that does not advance would read the page again
            // forever.
            if next.is_some() && next == token {
                return Err(Error::query("BigQuery returned the same page token twice"));
            }
            token = next;
            if complete && token.is_none() {
                stop.finish();
                return Ok(sent);
            }
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

    /// The classes of the table's columns, for a filter that compares a
    /// value against them: bytes read as the bytes their text names, and
    /// a number or a boolean written bare, since the server matches no
    /// string to one. See [`Dialect::select_rows`]. Fetched only when a
    /// filter compares.
    async fn column_classes(&self, query: &RowQuery) -> Result<Vec<(String, ColumnClass)>> {
        let compares = query.filters.iter().any(|filter| match filter.op {
            FilterOp::Eq
            | FilterOp::Ne
            | FilterOp::In
            | FilterOp::Lt
            | FilterOp::Gt
            | FilterOp::Le
            | FilterOp::Ge => true,
            FilterOp::Contains | FilterOp::StartsWith | FilterOp::IsNull | FilterOp::IsNotNull => {
                false
            }
        });
        if !compares {
            return Ok(Vec::new());
        }
        Ok(self
            .fields(&query.object)
            .await?
            .iter()
            .map(|field| {
                (
                    field.name.clone(),
                    crate::column_class(Dialect::BigQuery, &type_name(field)),
                )
            })
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
        let mut token: Option<String> = None;
        loop {
            let mut options = gcp_bigquery_client::dataset::ListOptions::default();
            if let Some(token) = &token {
                options = options.page_token(token.clone());
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
                Some(next) if token.as_deref() == Some(next.as_str()) => {
                    return Err(Error::query("BigQuery returned the same page token twice"));
                }
                Some(next) => token = Some(next),
                None => return Ok(names),
            }
        }
    }

    /// The dataset's tables and views, as objects.
    async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let mut objects = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut options = gcp_bigquery_client::table::ListOptions::default();
            if let Some(token) = &token {
                options = options.page_token(token.clone());
            }
            let page = self
                .client
                .table()
                .list(&self.project, schema, options)
                .await
                .map_err(query_error)?;
            objects.extend(
                page.tables
                    .unwrap_or_default()
                    .into_iter()
                    .map(|table| ObjectInfo {
                        name: table.table_reference.table_id,
                        kind: match table.r#type.as_deref() {
                            Some("VIEW") => ObjectKind::View,
                            Some("MATERIALIZED_VIEW") => ObjectKind::MaterializedView,
                            _ => ObjectKind::Table,
                        },
                        estimated_rows: None,
                    }),
            );
            if objects.len() >= MAX_LISTED as usize {
                objects.truncate(MAX_LISTED as usize);
                return Ok(objects);
            }
            match page.next_page_token {
                Some(next) if token.as_deref() == Some(next.as_str()) => {
                    return Err(Error::query("BigQuery returned the same page token twice"));
                }
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
        let classes = self.column_classes(query).await?;
        let sql = Dialect::BigQuery.select_rows(query, &[], &classes)?;
        debug_assert!(sql.params.is_empty());
        let limit = query.limit as usize;
        let (columns, mut rows, elapsed) = self.query(&sql.text, &[], limit + 1, None).await?;
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

    async fn export_statement(&self, query: &RowQuery, columns: &[String]) -> Result<String> {
        let classes = self.column_classes(query).await?;
        Dialect::BigQuery.shown_export(query, &[], &classes, columns)
    }

    /// An export's rows through the jobs API, a page (at most `batch`
    /// rows) held at a time: there is no cursor, so the API's own paging
    /// is the stream.
    async fn stream_rows(
        &self,
        query: &RowQuery,
        columns: &[String],
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let classes = self.column_classes(query).await?;
        let sql = Dialect::BigQuery.export_rows(query, &[], &classes, columns)?;
        debug_assert!(sql.params.is_empty());
        self.stream_read(&sql.text, batch, to, stop).await
    }

    /// A statement of the person's, as `stream_rows` streams a table's.
    async fn stream_statement(
        &self,
        text: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        self.stream_read(text, batch, to, stop).await
    }

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let classes = self.column_classes(query).await?;
        let sql = Dialect::BigQuery.count_rows(query, &classes)?;
        let (_, rows, _) = self.query(&sql.text, &[], 1, None).await?;
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
        let (_, rows, _) = self
            .query(&sql.text, &[], lookup.limit as usize + 1, None)
            .await?;
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
                let (columns, rows, _) = self.query(&text, &[], limit as usize + 1, None).await?;
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
            .query(
                read.text(),
                read.params(),
                limits.rows as usize + 1,
                Some(limits.timeout),
            )
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
    fn job_ids_carry_the_process_and_its_run() {
        let one = next_job_id();
        let another = next_job_id();
        assert_ne!(one, another);
        // tabletist-{pid}-{run as hex}-{counter}
        assert_eq!(one.split('-').count(), 4);
        assert!(one.starts_with(&format!("tabletist-{}-", std::process::id())));
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
            cell(
                &field(FieldType::Datetime, None),
                &json("2024-02-03T04:05:06")
            ),
            Value::Text("2024-02-03 04:05:06".into())
        );
        // Repeated and nested values show as the JSON the API gives.
        let array = serde_json::json!(["a", "b"]);
        assert_eq!(
            cell(&field(FieldType::String, Some("REPEATED")), &array),
            Value::Text(r#"["a","b"]"#.into())
        );
        assert_eq!(
            type_name(&field(FieldType::String, Some("REPEATED"))),
            "ARRAY<STRING>"
        );
        assert_eq!(type_name(&field(FieldType::Int64, None)), "INT64");
    }

    #[test]
    fn a_raw_where_cannot_smuggle_a_second_statement() {
        // What fetch_rows and count_rows build with a hostile raw WHERE,
        // then send through query()'s check.
        let mut q = crate::RowQuery::new(crate::ObjectRef::new("d", "t"), 100);
        q.raw_where = Some("1 = 1\n) ; DELETE FROM d.t WHERE true; --".into());
        let sql = Dialect::BigQuery
            .select_rows(&q, &[], &[])
            .expect("no filter, nothing to parse");
        assert!(
            crate::agent::check_read(crate::Dialect::BigQuery, &sql.text, &[]).is_err(),
            "{}",
            sql.text
        );
    }

    #[test]
    fn a_script_only_runs_what_proves_a_read() {
        // The guard the adapter's run_script and stream_read (a table's or
        // a statement's export) stand on.
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
            "SELECT 1; DELETE FROM d.t WHERE true",
        ] {
            assert!(
                crate::agent::check_read(crate::Dialect::BigQuery, text, &[]).is_err(),
                "{text} must be refused"
            );
        }
    }
}
