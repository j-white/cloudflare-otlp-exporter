use crate::metrics::prometheus_registry_to_opentelemetry_metrics;
use chrono::NaiveDateTime;
use graphql_client::{GraphQLQuery, Response};
use opentelemetry_proto::tonic::metrics::v1::Metric;
use prometheus::{CounterVec, GaugeVec, Opts, Registry};
use std::error::Error;
use worker::console_log;

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the schema
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/workers_query.graphql"
)]
pub struct GetWorkersAnalyticsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/d1_query.graphql"
)]
pub struct GetD1AnalyticsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/durableobjects_query.graphql"
)]
pub struct GetDurableObjectsAnalyticsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/queue_backlog_query.graphql"
)]
pub struct GetQueueBacklogAnalyticsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/queue_operations_query.graphql"
)]
pub struct GetQueueOperationsAnalyticsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/zone_http_requests_query.graphql"
)]
pub struct GetZoneHttpRequestsQuery;

#[allow(non_camel_case_types)]
type float32 = f32;

#[allow(non_camel_case_types)]
type string = String;

#[allow(non_camel_case_types)]
type Time = String;

#[allow(non_camel_case_types)]
type uint64 = u64;

#[allow(non_camel_case_types)]
type uint32 = u32;

#[allow(non_camel_case_types)]
type float64 = f64;

#[allow(non_camel_case_types)]
type uint16 = u16;

pub async fn do_get_workers_analytics_query(
    cloudflare_api_url: &String,
    cloudflare_api_key: &String,
    variables: get_workers_analytics_query::Variables,
    debug_logging: bool,
    fallback_timestamp_nanos: u64,
) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetWorkersAnalyticsQuery::build_query(variables);
    if debug_logging {
        console_log!(
            "[Workers] GraphQL request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );
    }
    let client = reqwest::Client::new();
    let res = client
        .post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        console_log!("[Workers] GraphQL query failed: {:?}", res.status());
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_text = res.text().await?;
    if debug_logging {
        console_log!("[Workers] GraphQL response: {}", response_text);
    }
    let response_body: Response<get_workers_analytics_query::ResponseData> =
        serde_json::from_str(&response_text)?;
    if response_body.errors.is_some() {
        console_log!("[Workers] GraphQL query failed: {:?}", response_body.errors);
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_workers_analytics_query::ResponseData =
        response_body.data.expect("missing response data");

    let registry = Registry::new();
    let worker_requests_opts = Opts::new("cloudflare_worker_requests", "Sum of Requests");
    let worker_requests = CounterVec::new(worker_requests_opts, &["script_name"]).unwrap();
    registry
        .register(Box::new(worker_requests.clone()))
        .unwrap();

    let worker_errors_opts = Opts::new("cloudflare_worker_errors", "Sum of Errors");
    let worker_errors = CounterVec::new(worker_errors_opts, &["script_name"]).unwrap();
    registry.register(Box::new(worker_errors.clone())).unwrap();

    let worker_cpu_time_opts = Opts::new("cloudflare_worker_cpu_time", "CPU time - microseconds");
    let worker_cpu_time =
        GaugeVec::new(worker_cpu_time_opts, &["script_name", "quantile"]).unwrap();
    registry
        .register(Box::new(worker_cpu_time.clone()))
        .unwrap();

    let worker_duration_opts = Opts::new("cloudflare_worker_duration", "Duration - GB*s");
    let worker_duration =
        GaugeVec::new(worker_duration_opts, &["script_name", "quantile"]).unwrap();
    registry
        .register(Box::new(worker_duration.clone()))
        .unwrap();

    let worker_wall_time_opts = Opts::new(
        "cloudflare_worker_wall_time",
        "Sum of wall time - microseconds",
    );
    let worker_wall_time = CounterVec::new(worker_wall_time_opts, &["script_name"]).unwrap();
    registry
        .register(Box::new(worker_wall_time.clone()))
        .unwrap();

    let worker_subrequests_opts = Opts::new("cloudflare_worker_subrequests", "Sum of subrequests");
    let worker_subrequests = CounterVec::new(worker_subrequests_opts, &["script_name"]).unwrap();
    registry
        .register(Box::new(worker_subrequests.clone()))
        .unwrap();

    let mut last_datetime: Option<Time> = None;
    for account in response_data.viewer.unwrap().accounts.iter() {
        for worker in account.workers_invocations_adaptive.iter() {
            let dimensions = worker.dimensions.as_ref().unwrap();
            last_datetime = Some(dimensions.datetime.clone());
            let script_name = dimensions.script_name.clone();
            let sum = worker.sum.as_ref().unwrap();
            let quantiles = worker.quantiles.as_ref().unwrap();

            worker_requests
                .with_label_values(&[script_name.as_str()])
                .inc_by(sum.requests as f64);
            worker_errors
                .with_label_values(&[script_name.as_str()])
                .inc_by(sum.errors as f64);
            worker_wall_time
                .with_label_values(&[script_name.as_str()])
                .inc_by(sum.wall_time as f64);
            worker_subrequests
                .with_label_values(&[script_name.as_str()])
                .inc_by(sum.subrequests as f64);
            worker_cpu_time
                .with_label_values(&[script_name.as_str(), "P50"])
                .set(quantiles.cpu_time_p50 as f64);
            worker_cpu_time
                .with_label_values(&[script_name.as_str(), "P75"])
                .set(quantiles.cpu_time_p75 as f64);
            worker_cpu_time
                .with_label_values(&[script_name.as_str(), "P99"])
                .set(quantiles.cpu_time_p99 as f64);
            worker_cpu_time
                .with_label_values(&[script_name.as_str(), "P999"])
                .set(quantiles.cpu_time_p999 as f64);
            worker_duration
                .with_label_values(&[script_name.as_str(), "P50"])
                .set(quantiles.duration_p50 as f64);
            worker_duration
                .with_label_values(&[script_name.as_str(), "P75"])
                .set(quantiles.duration_p75 as f64);
            worker_duration
                .with_label_values(&[script_name.as_str(), "P99"])
                .set(quantiles.duration_p99 as f64);
            worker_duration
                .with_label_values(&[script_name.as_str(), "P999"])
                .set(quantiles.duration_p999 as f64);
        }
    }

    let timestamp_nanos: u64 = last_datetime
        .map(|datetime| {
            let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&datetime, "%+").unwrap();
            datetime.and_utc().timestamp_nanos_opt().unwrap_or(0) as u64
        })
        .unwrap_or(fallback_timestamp_nanos);

    Ok(prometheus_registry_to_opentelemetry_metrics(
        registry,
        timestamp_nanos,
    ))
}

pub async fn do_get_d1_analytics_query(
    cloudflare_api_url: &String,
    cloudflare_api_key: &String,
    variables: get_d1_analytics_query::Variables,
    debug_logging: bool,
    fallback_timestamp_nanos: u64,
) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetD1AnalyticsQuery::build_query(variables);
    if debug_logging {
        console_log!(
            "[D1] GraphQL request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );
    }
    let client = reqwest::Client::new();
    let res = client
        .post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        console_log!("[D1] GraphQL query failed: {:?}", res.status());
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_text = res.text().await?;
    if debug_logging {
        console_log!("[D1] GraphQL response: {}", response_text);
    }
    let response_body: Response<get_d1_analytics_query::ResponseData> =
        serde_json::from_str(&response_text)?;
    if response_body.errors.is_some() {
        console_log!("[D1] GraphQL query failed: {:?}", response_body.errors);
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_d1_analytics_query::ResponseData =
        response_body.data.expect("missing response data");

    let registry = Registry::new();
    let d1_read_queries_opts =
        Opts::new("cloudflare_d1_read_queries", "The number of read queries.");
    let d1_read_queries = CounterVec::new(d1_read_queries_opts, &["database_id"]).unwrap();
    registry
        .register(Box::new(d1_read_queries.clone()))
        .unwrap();

    let d1_rows_read_opts = Opts::new(
        "cloudflare_d1_rows_read",
        "The number of rows your queries read.",
    );
    let d1_rows_read = CounterVec::new(d1_rows_read_opts, &["database_id"]).unwrap();
    registry.register(Box::new(d1_rows_read.clone())).unwrap();

    let d1_rows_written_opts = Opts::new(
        "cloudflare_d1_rows_written",
        "The number of rows your queries wrote.",
    );
    let d1_rows_written = CounterVec::new(d1_rows_written_opts, &["database_id"]).unwrap();
    registry
        .register(Box::new(d1_rows_written.clone()))
        .unwrap();

    let d1_write_queries_opts = Opts::new(
        "cloudflare_d1_write_queries",
        "The number of write queries.",
    );
    let d1_write_queries = CounterVec::new(d1_write_queries_opts, &["database_id"]).unwrap();
    registry
        .register(Box::new(d1_write_queries.clone()))
        .unwrap();

    let d1_query_batch_response_bytes_opts = Opts::new(
        "cloudflare_d1_query_batch_response_bytes",
        "The total number of bytes in the response, including all returned rows and metadata.",
    );
    let d1_query_batch_response_bytes = GaugeVec::new(
        d1_query_batch_response_bytes_opts,
        &["database_id", "quantile"],
    )
    .unwrap();
    registry
        .register(Box::new(d1_query_batch_response_bytes.clone()))
        .unwrap();

    let d1_query_batch_time_ms_opts = Opts::new(
        "cloudflare_d1_query_batch_time_ms",
        "Query batch response time in milliseconds.",
    );
    let d1_query_batch_time_ms =
        GaugeVec::new(d1_query_batch_time_ms_opts, &["database_id", "quantile"]).unwrap();
    registry
        .register(Box::new(d1_query_batch_time_ms.clone()))
        .unwrap();

    let mut last_datetime: Option<Time> = None;
    for account in response_data.viewer.unwrap().accounts.iter() {
        for group in account.d1_analytics_adaptive_groups.iter() {
            let dimensions = group.dimensions.as_ref().unwrap();
            last_datetime = Some(dimensions.datetime_minute.clone());
            let database_id = dimensions.database_id.clone();
            let sum = group.sum.as_ref().unwrap();
            let quantiles = group.quantiles.as_ref().unwrap();

            d1_read_queries
                .with_label_values(&[database_id.as_str()])
                .inc_by(sum.read_queries as f64);
            d1_rows_read
                .with_label_values(&[database_id.as_str()])
                .inc_by(sum.rows_read as f64);
            d1_rows_written
                .with_label_values(&[database_id.as_str()])
                .inc_by(sum.rows_written as f64);
            d1_write_queries
                .with_label_values(&[database_id.as_str()])
                .inc_by(sum.write_queries as f64);

            d1_query_batch_response_bytes
                .with_label_values(&[database_id.as_str(), "P50"])
                .set(quantiles.query_batch_response_bytes_p50);
            d1_query_batch_response_bytes
                .with_label_values(&[database_id.as_str(), "P90"])
                .set(quantiles.query_batch_response_bytes_p90);
            d1_query_batch_time_ms
                .with_label_values(&[database_id.as_str(), "P50"])
                .set(quantiles.query_batch_time_ms_p50);
            d1_query_batch_time_ms
                .with_label_values(&[database_id.as_str(), "P90"])
                .set(quantiles.query_batch_time_ms_p90);
        }
    }

    let timestamp_nanos: u64 = last_datetime
        .map(|datetime| {
            let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&datetime, "%+").unwrap();
            datetime.and_utc().timestamp_nanos_opt().unwrap_or(0) as u64
        })
        .unwrap_or(fallback_timestamp_nanos);

    Ok(prometheus_registry_to_opentelemetry_metrics(
        registry,
        timestamp_nanos,
    ))
}

pub async fn do_get_durableobjects_analytics_query(
    cloudflare_api_url: &String,
    cloudflare_api_key: &String,
    variables: get_durable_objects_analytics_query::Variables,
    debug_logging: bool,
    fallback_timestamp_nanos: u64,
) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetDurableObjectsAnalyticsQuery::build_query(variables);
    if debug_logging {
        console_log!(
            "[DurableObjects] GraphQL request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );
    }
    let client = reqwest::Client::new();
    let res = client
        .post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        console_log!("[DurableObjects] GraphQL query failed: {:?}", res.status());
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_text = res.text().await?;
    if debug_logging {
        console_log!("[DurableObjects] GraphQL response: {}", response_text);
    }
    let response_body: Response<get_durable_objects_analytics_query::ResponseData> =
        serde_json::from_str(&response_text)?;
    if response_body.errors.is_some() {
        console_log!(
            "[DurableObjects] GraphQL query failed: {:?}",
            response_body.errors
        );
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_durable_objects_analytics_query::ResponseData =
        response_body.data.expect("missing response data");

    let registry = Registry::new();
    let do_errors_opts = Opts::new("cloudflare_durable_objects_errors", "Sum of errors");
    let do_errors = CounterVec::new(do_errors_opts, &["script_name"]).unwrap();
    registry.register(Box::new(do_errors.clone())).unwrap();

    let do_requests_opts = Opts::new("cloudflare_durable_objects_requests", "Sum of requests");
    let do_requests = CounterVec::new(do_requests_opts, &["script_name"]).unwrap();
    registry.register(Box::new(do_requests.clone())).unwrap();

    let do_response_body_size_bytes_opts = Opts::new(
        "cloudflare_durable_objects_response_body_size_bytes",
        "Response body size - bytes",
    );
    let do_response_body_size_bytes = GaugeVec::new(
        do_response_body_size_bytes_opts,
        &["script_name", "quantile"],
    )
    .unwrap();
    registry
        .register(Box::new(do_response_body_size_bytes.clone()))
        .unwrap();

    let do_wall_time_microseconds_opts = Opts::new(
        "cloudflare_durable_objects_wall_time_microseconds",
        "Wall time - microseconds",
    );
    let do_wall_time_microseconds =
        GaugeVec::new(do_wall_time_microseconds_opts, &["script_name", "quantile"]).unwrap();
    registry
        .register(Box::new(do_wall_time_microseconds.clone()))
        .unwrap();

    let mut last_datetime: Option<Time> = None;
    for account in response_data.viewer.unwrap().accounts.iter() {
        for group in account.durable_objects_invocations_adaptive_groups.iter() {
            let dimensions = group.dimensions.as_ref().unwrap();
            last_datetime = Some(dimensions.datetime_minute.clone());
            let script_name = dimensions.script_name.clone();
            let sum = group.sum.as_ref().unwrap();
            let quantiles = group.quantiles.as_ref().unwrap();

            do_errors
                .with_label_values(&[script_name.as_str()])
                .inc_by(sum.errors as f64);
            do_requests
                .with_label_values(&[script_name.as_str()])
                .inc_by(sum.requests as f64);

            do_response_body_size_bytes
                .with_label_values(&[script_name.as_str(), "P25"])
                .set(quantiles.response_body_size_p25 as f64);
            do_response_body_size_bytes
                .with_label_values(&[script_name.as_str(), "P50"])
                .set(quantiles.response_body_size_p50 as f64);
            do_response_body_size_bytes
                .with_label_values(&[script_name.as_str(), "P75"])
                .set(quantiles.response_body_size_p75 as f64);
            do_response_body_size_bytes
                .with_label_values(&[script_name.as_str(), "P90"])
                .set(quantiles.response_body_size_p90 as f64);
            do_response_body_size_bytes
                .with_label_values(&[script_name.as_str(), "P99"])
                .set(quantiles.response_body_size_p99 as f64);
            do_response_body_size_bytes
                .with_label_values(&[script_name.as_str(), "P999"])
                .set(quantiles.response_body_size_p999 as f64);

            do_wall_time_microseconds
                .with_label_values(&[script_name.as_str(), "P25"])
                .set(quantiles.wall_time_p25 as f64);
            do_wall_time_microseconds
                .with_label_values(&[script_name.as_str(), "P50"])
                .set(quantiles.wall_time_p50 as f64);
            do_wall_time_microseconds
                .with_label_values(&[script_name.as_str(), "P75"])
                .set(quantiles.wall_time_p75 as f64);
            do_wall_time_microseconds
                .with_label_values(&[script_name.as_str(), "P90"])
                .set(quantiles.wall_time_p90 as f64);
            do_wall_time_microseconds
                .with_label_values(&[script_name.as_str(), "P99"])
                .set(quantiles.wall_time_p99 as f64);
            do_wall_time_microseconds
                .with_label_values(&[script_name.as_str(), "P999"])
                .set(quantiles.wall_time_p999 as f64);
        }
    }

    let timestamp_nanos: u64 = last_datetime
        .map(|datetime| {
            let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&datetime, "%+").unwrap();
            datetime.and_utc().timestamp_nanos_opt().unwrap_or(0) as u64
        })
        .unwrap_or(fallback_timestamp_nanos);

    Ok(prometheus_registry_to_opentelemetry_metrics(
        registry,
        timestamp_nanos,
    ))
}

pub async fn do_get_queue_backlog_analytics_query(
    cloudflare_api_url: &String,
    cloudflare_api_key: &String,
    variables: get_queue_backlog_analytics_query::Variables,
    debug_logging: bool,
    fallback_timestamp_nanos: u64,
) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetQueueBacklogAnalyticsQuery::build_query(variables);
    if debug_logging {
        console_log!(
            "[QueueBacklog] GraphQL request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );
    }
    let client = reqwest::Client::new();
    let res = client
        .post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        console_log!("[QueueBacklog] GraphQL query failed: {:?}", res.status());
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_text = res.text().await?;
    if debug_logging {
        console_log!("[QueueBacklog] GraphQL response: {}", response_text);
    }
    let response_body: Response<get_queue_backlog_analytics_query::ResponseData> =
        serde_json::from_str(&response_text)?;
    if response_body.errors.is_some() {
        console_log!(
            "[QueueBacklog] GraphQL query failed: {:?}",
            response_body.errors
        );
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_queue_backlog_analytics_query::ResponseData =
        response_body.data.expect("missing response data");

    let registry = Registry::new();
    let queue_backlog_bytes_opts = Opts::new(
        "cloudflare_queue_backlog_bytes",
        "The average size of the backlog in bytes for sample interval",
    );
    let queue_backlog_bytes = GaugeVec::new(queue_backlog_bytes_opts, &["queue_id"]).unwrap();
    registry
        .register(Box::new(queue_backlog_bytes.clone()))
        .unwrap();

    let queue_backlog_messages_opts = Opts::new(
        "cloudflare_queue_backlog_messages",
        "The average number of messages in the backlog for sample interval",
    );
    let queue_backlog_messages = GaugeVec::new(queue_backlog_messages_opts, &["queue_id"]).unwrap();
    registry
        .register(Box::new(queue_backlog_messages.clone()))
        .unwrap();

    let queue_backlog_sample_interval_opts = Opts::new(
        "cloudflare_queue_backlog_sample_interval",
        "The average value used for sample interval",
    );
    let queue_backlog_sample_interval =
        GaugeVec::new(queue_backlog_sample_interval_opts, &["queue_id"]).unwrap();
    registry
        .register(Box::new(queue_backlog_sample_interval.clone()))
        .unwrap();

    let mut last_datetime: Option<Time> = None;
    for account in response_data.viewer.unwrap().accounts.iter() {
        for group in account.queue_backlog_adaptive_groups.iter() {
            let dimensions = group.dimensions.as_ref().unwrap();
            last_datetime = Some(dimensions.datetime_minute.clone());
            let queue_id = dimensions.queue_id.clone();
            let avg = group.avg.as_ref().unwrap();

            queue_backlog_bytes
                .with_label_values(&[queue_id.as_str()])
                .set(avg.bytes as f64);
            queue_backlog_messages
                .with_label_values(&[queue_id.as_str()])
                .set(avg.messages as f64);
            queue_backlog_sample_interval
                .with_label_values(&[queue_id.as_str()])
                .set(avg.sample_interval);
        }
    }

    let timestamp_nanos: u64 = last_datetime
        .map(|datetime| {
            let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&datetime, "%+").unwrap();
            datetime.and_utc().timestamp_nanos_opt().unwrap_or(0) as u64
        })
        .unwrap_or(fallback_timestamp_nanos);

    Ok(prometheus_registry_to_opentelemetry_metrics(
        registry,
        timestamp_nanos,
    ))
}

pub async fn do_get_queue_operations_analytics_query(
    cloudflare_api_url: &String,
    cloudflare_api_key: &String,
    variables: get_queue_operations_analytics_query::Variables,
    debug_logging: bool,
    fallback_timestamp_nanos: u64,
) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetQueueOperationsAnalyticsQuery::build_query(variables);
    if debug_logging {
        console_log!(
            "[QueueOperations] GraphQL request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );
    }
    let client = reqwest::Client::new();
    let res = client
        .post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        console_log!("[QueueOperations] GraphQL query failed: {:?}", res.status());
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_text = res.text().await?;
    if debug_logging {
        console_log!("[QueueOperations] GraphQL response: {}", response_text);
    }
    let response_body: Response<get_queue_operations_analytics_query::ResponseData> =
        serde_json::from_str(&response_text)?;
    if response_body.errors.is_some() {
        console_log!(
            "[QueueOperations] GraphQL query failed: {:?}",
            response_body.errors
        );
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_queue_operations_analytics_query::ResponseData =
        response_body.data.expect("missing response data");

    let registry = Registry::new();
    let queue_billable_opts = Opts::new("cloudflare_queue_operations_billable", "Number of Billable Operations (some message operations count as multiple billable operations)");
    let queue_billable = CounterVec::new(
        queue_billable_opts,
        &["action_type", "consumer_type", "queue_id", "outcome"],
    )
    .unwrap();
    registry.register(Box::new(queue_billable.clone())).unwrap();

    let queue_lag_time_ms_opts = Opts::new("cloudflare_queue_operations_lag_time_ms", "The average time in milliseconds between when the message was written to the queue and the current operation over the sample interval. Will always be 0 for WriteMessage operations.");
    let queue_lag_time_ms = GaugeVec::new(
        queue_lag_time_ms_opts,
        &["action_type", "consumer_type", "queue_id", "outcome"],
    )
    .unwrap();
    registry
        .register(Box::new(queue_lag_time_ms.clone()))
        .unwrap();

    let queue_retry_count_opts = Opts::new("cloudflare_queue_operations_retry_count", "The average number of retries per message operation. A retry occurs after an unsucessful delivery, if the queue is configured to retry failed attempts. Only applicable to ReadMessage and DeleteMessage operations. Will always be 0 for WriteMessage operations.");
    let queue_retry_count = GaugeVec::new(
        queue_retry_count_opts,
        &["action_type", "consumer_type", "queue_id", "outcome"],
    )
    .unwrap();
    registry
        .register(Box::new(queue_retry_count.clone()))
        .unwrap();

    let queue_sample_interval_opts = Opts::new(
        "cloudflare_queue_operations_sample_interval",
        "The average value used for sample interval",
    );
    let queue_sample_interval = GaugeVec::new(
        queue_sample_interval_opts,
        &["action_type", "consumer_type", "queue_id", "outcome"],
    )
    .unwrap();
    registry
        .register(Box::new(queue_sample_interval.clone()))
        .unwrap();

    let mut last_datetime: Option<Time> = None;
    for account in response_data.viewer.unwrap().accounts.iter() {
        for group in account.queue_message_operations_adaptive_groups.iter() {
            let dimensions = group.dimensions.as_ref().unwrap();
            last_datetime = Some(dimensions.datetime.clone());
            let action_type = dimensions.action_type.clone();
            let consumer_type = dimensions.consumer_type.clone();
            let queue_id = dimensions.queue_id.clone();
            let outcome = dimensions.outcome.clone();

            let sum = group.sum.as_ref().unwrap();
            let avg = group.avg.as_ref().unwrap();

            queue_billable
                .with_label_values(&[
                    action_type.as_str(),
                    consumer_type.as_str(),
                    queue_id.as_str(),
                    outcome.as_str(),
                ])
                .inc_by(sum.billable_operations as f64);

            queue_lag_time_ms
                .with_label_values(&[
                    action_type.as_str(),
                    consumer_type.as_str(),
                    queue_id.as_str(),
                    outcome.as_str(),
                ])
                .set(avg.lag_time as f64);
            queue_retry_count
                .with_label_values(&[
                    action_type.as_str(),
                    consumer_type.as_str(),
                    queue_id.as_str(),
                    outcome.as_str(),
                ])
                .set(avg.retry_count as f64);
            queue_sample_interval
                .with_label_values(&[
                    action_type.as_str(),
                    consumer_type.as_str(),
                    queue_id.as_str(),
                    outcome.as_str(),
                ])
                .set(avg.sample_interval);
        }
    }

    let timestamp_nanos: u64 = last_datetime
        .map(|datetime| {
            let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&datetime, "%+").unwrap();
            datetime.and_utc().timestamp_nanos_opt().unwrap_or(0) as u64
        })
        .unwrap_or(fallback_timestamp_nanos);

    Ok(prometheus_registry_to_opentelemetry_metrics(
        registry,
        timestamp_nanos,
    ))
}

pub async fn do_get_zone_http_requests_query(
    cloudflare_api_url: &String,
    cloudflare_api_key: &String,
    variables: get_zone_http_requests_query::Variables,
    debug_logging: bool,
    fallback_timestamp_nanos: u64,
) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetZoneHttpRequestsQuery::build_query(variables);
    if debug_logging {
        console_log!(
            "[ZoneHttpRequests] GraphQL request: {}",
            serde_json::to_string_pretty(&request_body).unwrap_or_default()
        );
    }
    let client = reqwest::Client::new();
    let res = client
        .post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        console_log!(
            "[ZoneHttpRequests] GraphQL query failed: {:?}",
            res.status()
        );
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_text = res.text().await?;
    if debug_logging {
        console_log!("[ZoneHttpRequests] GraphQL response: {}", response_text);
    }
    let response_body: Response<get_zone_http_requests_query::ResponseData> =
        serde_json::from_str(&response_text)?;
    if response_body.errors.is_some() {
        console_log!(
            "[ZoneHttpRequests] GraphQL query failed: {:?}",
            response_body.errors
        );
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_zone_http_requests_query::ResponseData =
        response_body.data.expect("missing response data");

    let registry = Registry::new();
    let zone_requests_status_host_opts = Opts::new(
        "cloudflare_zone_requests_status_host",
        "Count of requests per edge HTTP status per host",
    );
    let zone_requests_status_host =
        CounterVec::new(zone_requests_status_host_opts, &["zone", "status", "host"]).unwrap();
    registry
        .register(Box::new(zone_requests_status_host.clone()))
        .unwrap();

    let zone_ttfb_opts = Opts::new(
        "cloudflare_zone_edge_ttfb_ms",
        "Edge Time To First Byte - milliseconds",
    );
    let zone_ttfb = GaugeVec::new(zone_ttfb_opts, &["zone", "host", "quantile"]).unwrap();
    registry.register(Box::new(zone_ttfb.clone())).unwrap();

    let zone_origin_response_duration_opts = Opts::new(
        "cloudflare_zone_origin_response_duration_ms",
        "Origin Response Duration - milliseconds",
    );
    let zone_origin_response_duration = GaugeVec::new(
        zone_origin_response_duration_opts,
        &["zone", "host", "quantile"],
    )
    .unwrap();
    registry
        .register(Box::new(zone_origin_response_duration.clone()))
        .unwrap();

    let last_datetime: Option<Time> = None;
    for zone in response_data.viewer.unwrap().zones.iter() {
        let zone_tag = zone.zone_tag.clone();
        for group in zone.http_requests_adaptive_groups.iter() {
            let dimensions = group.dimensions.as_ref().unwrap();
            let status = dimensions.edge_response_status.to_string();
            let host = dimensions.client_request_http_host.clone();
            let count = group.count;

            zone_requests_status_host
                .with_label_values(&[zone_tag.as_str(), status.as_str(), host.as_str()])
                .inc_by(count as f64);

            if let Some(quantiles) = &group.quantiles {
                zone_ttfb
                    .with_label_values(&[zone_tag.as_str(), host.as_str(), "P50"])
                    .set(quantiles.edge_time_to_first_byte_ms_p50);
                zone_ttfb
                    .with_label_values(&[zone_tag.as_str(), host.as_str(), "P95"])
                    .set(quantiles.edge_time_to_first_byte_ms_p95);
                zone_ttfb
                    .with_label_values(&[zone_tag.as_str(), host.as_str(), "P99"])
                    .set(quantiles.edge_time_to_first_byte_ms_p99);

                zone_origin_response_duration
                    .with_label_values(&[zone_tag.as_str(), host.as_str(), "P50"])
                    .set(quantiles.origin_response_duration_ms_p50);
                zone_origin_response_duration
                    .with_label_values(&[zone_tag.as_str(), host.as_str(), "P95"])
                    .set(quantiles.origin_response_duration_ms_p95);
                zone_origin_response_duration
                    .with_label_values(&[zone_tag.as_str(), host.as_str(), "P99"])
                    .set(quantiles.origin_response_duration_ms_p99);
            }
        }
    }

    let timestamp_nanos: u64 = last_datetime
        .map(|datetime| {
            let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&datetime, "%+").unwrap();
            datetime.and_utc().timestamp_nanos_opt().unwrap_or(0) as u64
        })
        .unwrap_or(fallback_timestamp_nanos);

    Ok(prometheus_registry_to_opentelemetry_metrics(
        registry,
        timestamp_nanos,
    ))
}
