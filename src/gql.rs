use std::error::Error;
use graphql_client::{GraphQLQuery, Response};
use opentelemetry_sdk::metrics::data::Metric;
use prometheus::{CounterVec, GaugeVec, Opts, Registry};
use crate::metrics::prometheus_registry_to_opentelemetry_metrics;
use web_time::SystemTime;
use chrono::NaiveDateTime;
use worker::console_log;

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the schema
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/workers_query.graphql",
    variables_derives = "Debug",
    response_derives = "Debug,Clone"
)]
pub struct GetWorkersAnalyticsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/d1_query.graphql",
    variables_derives = "Debug",
    response_derives = "Debug,Clone"
)]
pub struct GetD1AnalyticsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/durableobjects_query.graphql",
    variables_derives = "Debug",
    response_derives = "Debug,Clone"
)]
pub struct GetDurableObjectsAnalyticsQuery;

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "gql/schema.graphql",
//     query_path = "gql/queries.graphql",
//     variables_derives = "Debug",
//     response_derives = "Debug,Clone"
// )]
// pub struct GetQueueAnalyticsQuery;

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

pub async fn do_get_workers_analytics_query(cloudflare_api_url: &String, cloudflare_api_key: &String, variables: get_workers_analytics_query::Variables) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetWorkersAnalyticsQuery::build_query(variables);
    //console_log!("request_body: {:?}", request_body);
    let client = reqwest::Client::new();
    let res = client.post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body).send().await?;

    if !res.status().is_success() {
        console_log!("GraphQL query failed: {:?}", res.status());
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_body: Response<get_workers_analytics_query::ResponseData> = res.json().await?;
    if response_body.errors.is_some() {
        console_log!("GraphQL query failed: {:?}", response_body.errors);
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_workers_analytics_query::ResponseData = response_body.data.expect("missing response data");

    let registry = Registry::new();
    let worker_requests_opts = Opts::new("cloudflare_worker_requests", "Sum of Requests");
    let worker_requests = CounterVec::new(worker_requests_opts, &["script_name"]).unwrap();
    registry.register(Box::new(worker_requests.clone())).unwrap();

    let worker_errors_opts = Opts::new("cloudflare_worker_errors", "Sum of Errors");
    let worker_errors = CounterVec::new(worker_errors_opts, &["script_name"]).unwrap();
    registry.register(Box::new(worker_errors.clone())).unwrap();

    let worker_cpu_time_opts = Opts::new("cloudflare_worker_cpu_time", "CPU time - microseconds");
    let worker_cpu_time = GaugeVec::new(worker_cpu_time_opts, &["script_name", "quantile"]).unwrap();
    registry.register(Box::new(worker_cpu_time.clone())).unwrap();

    let worker_duration_opts = Opts::new("cloudflare_worker_duration", "Duration - GB*s");
    let worker_duration = GaugeVec::new(worker_duration_opts, &["script_name", "quantile"]).unwrap();
    registry.register(Box::new(worker_duration.clone())).unwrap();

    let mut last_datetime: Option<Time> = None;
    for account in response_data.clone().viewer.unwrap().accounts.iter() {
        for worker in account.workers_invocations_adaptive.iter() {
            let dimensions = worker.dimensions.as_ref().unwrap();
            last_datetime = Some(dimensions.datetime.clone());
            let script_name = dimensions.script_name.clone();
            let sum = worker.sum.as_ref().unwrap();
            let quantiles = worker.quantiles.as_ref().unwrap();

            worker_requests.with_label_values(&[script_name.as_str()]).inc_by(sum.requests as f64);
            worker_errors.with_label_values(&[script_name.as_str()]).inc_by(sum.errors as f64);
            worker_cpu_time.with_label_values(&[script_name.as_str(), "P50"]).set(quantiles.cpu_time_p50 as f64);
            worker_cpu_time.with_label_values(&[script_name.as_str(), "P75"]).set(quantiles.cpu_time_p75 as f64);
            worker_cpu_time.with_label_values(&[script_name.as_str(), "P99"]).set(quantiles.cpu_time_p99 as f64);
            worker_cpu_time.with_label_values(&[script_name.as_str(), "P999"]).set(quantiles.cpu_time_p999 as f64);
            worker_duration.with_label_values(&[script_name.as_str(), "P50"]).set(quantiles.duration_p50 as f64);
            worker_duration.with_label_values(&[script_name.as_str(), "P75"]).set(quantiles.duration_p75 as f64);
            worker_duration.with_label_values(&[script_name.as_str(), "P99"]).set(quantiles.duration_p99 as f64);
            worker_duration.with_label_values(&[script_name.as_str(), "P999"]).set(quantiles.duration_p999 as f64);
        }
    }

    let timestamp: std::time::SystemTime = last_datetime.map(|datetime| {
        let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&*datetime, "%+").unwrap();
        datetime.and_utc().into()
    }).unwrap_or_else(|| {
        to_std_systemtime(SystemTime::now())
    });

    Ok(prometheus_registry_to_opentelemetry_metrics(registry, timestamp))
}

pub async fn do_get_d1_analytics_query(cloudflare_api_url: &String, cloudflare_api_key: &String, variables: get_d1_analytics_query::Variables) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetD1AnalyticsQuery::build_query(variables);
    //console_log!("request_body: {:?}", request_body);
    let client = reqwest::Client::new();
    let res = client.post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body).send().await?;

    if !res.status().is_success() {
        console_log!("GraphQL query failed: {:?}", res.status());
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_body: Response<get_d1_analytics_query::ResponseData> = res.json().await?;
    if response_body.errors.is_some() {
        console_log!("GraphQL query failed: {:?}", response_body.errors);
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_d1_analytics_query::ResponseData = response_body.data.expect("missing response data");

    let registry = Registry::new();
    let d1_read_queries_opts = Opts::new("cloudflare_d1_read_queries", "The number of read queries.");
    let d1_read_queries = CounterVec::new(d1_read_queries_opts, &["database_id"]).unwrap();
    registry.register(Box::new(d1_read_queries.clone())).unwrap();

    let d1_rows_read_opts = Opts::new("cloudflare_d1_rows_read", "The number of rows your queries read.");
    let d1_rows_read = CounterVec::new(d1_rows_read_opts, &["database_id"]).unwrap();
    registry.register(Box::new(d1_rows_read.clone())).unwrap();

    let d1_rows_written_opts = Opts::new("cloudflare_d1_rows_written", "The number of rows your queries wrote.");
    let d1_rows_written = CounterVec::new(d1_rows_written_opts, &["database_id"]).unwrap();
    registry.register(Box::new(d1_rows_written.clone())).unwrap();

    let d1_write_queries_opts = Opts::new("cloudflare_d1_write_queries", "The number of write queries.");
    let d1_write_queries = CounterVec::new(d1_write_queries_opts, &["database_id"]).unwrap();
    registry.register(Box::new(d1_write_queries.clone())).unwrap();

    let d1_query_batch_response_bytes_opts = Opts::new("cloudflare_d1_query_batch_response_bytes", "The total number of bytes in the response, including all returned rows and metadata.");
    let d1_query_batch_response_bytes = GaugeVec::new(d1_query_batch_response_bytes_opts, &["database_id", "quantile"]).unwrap();
    registry.register(Box::new(d1_query_batch_response_bytes.clone())).unwrap();

    let d1_query_batch_time_ms_opts = Opts::new("cloudflare_d1_query_batch_time_ms", "Query batch response time in milliseconds.");
    let d1_query_batch_time_ms = GaugeVec::new(d1_query_batch_time_ms_opts, &["database_id", "quantile"]).unwrap();
    registry.register(Box::new(d1_query_batch_time_ms.clone())).unwrap();

    let mut last_datetime: Option<Time> = None;
    for account in response_data.clone().viewer.unwrap().accounts.iter() {
        for group in account.d1_analytics_adaptive_groups.iter() {
            let dimensions = group.dimensions.as_ref().unwrap();
            last_datetime = Some(dimensions.datetime_minute.clone());
            let database_id = dimensions.database_id.clone();
            let sum = group.sum.as_ref().unwrap();
            let quantiles = group.quantiles.as_ref().unwrap();

            d1_read_queries.with_label_values(&[database_id.as_str()]).inc_by(sum.read_queries as f64);
            d1_rows_read.with_label_values(&[database_id.as_str()]).inc_by(sum.rows_read as f64);
            d1_rows_written.with_label_values(&[database_id.as_str()]).inc_by(sum.rows_written as f64);
            d1_write_queries.with_label_values(&[database_id.as_str()]).inc_by(sum.write_queries as f64);

            d1_query_batch_response_bytes.with_label_values(&[database_id.as_str(), "P50"]).set(quantiles.query_batch_response_bytes_p50 as f64);
            d1_query_batch_response_bytes.with_label_values(&[database_id.as_str(), "P90"]).set(quantiles.query_batch_response_bytes_p90 as f64);
            d1_query_batch_time_ms.with_label_values(&[database_id.as_str(), "P50"]).set(quantiles.query_batch_time_ms_p50 as f64);
            d1_query_batch_time_ms.with_label_values(&[database_id.as_str(), "P90"]).set(quantiles.query_batch_time_ms_p90 as f64);
        }
    }

    let timestamp: std::time::SystemTime = last_datetime.map(|datetime| {
        let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&*datetime, "%+").unwrap();
        datetime.and_utc().into()
    }).unwrap_or_else(|| {
        to_std_systemtime(SystemTime::now())
    });

    Ok(prometheus_registry_to_opentelemetry_metrics(registry, timestamp))
}

pub async fn do_get_durableobjects_analytics_query(cloudflare_api_url: &String, cloudflare_api_key: &String, variables: get_durable_objects_analytics_query::Variables) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetDurableObjectsAnalyticsQuery::build_query(variables);
    //console_log!("request_body: {:?}", request_body);
    let client = reqwest::Client::new();
    let res = client.post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body).send().await?;

    if !res.status().is_success() {
        console_log!("GraphQL query failed: {:?}", res.status());
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }

    let response_body: Response<get_durable_objects_analytics_query::ResponseData> = res.json().await?;
    if response_body.errors.is_some() {
        console_log!("GraphQL query failed: {:?}", response_body.errors);
        return Err(Box::new(worker::Error::JsError("graphql".parse().unwrap())));
    }
    let response_data: get_durable_objects_analytics_query::ResponseData = response_body.data.expect("missing response data");

    let registry = Registry::new();
    let do_errors_opts = Opts::new("cloudflare_durable_objects_errors", "Sum of errors");
    let do_errors = CounterVec::new(do_errors_opts, &["script_name"]).unwrap();
    registry.register(Box::new(do_errors.clone())).unwrap();

    let do_requests_opts = Opts::new("cloudflare_durable_objects_requests", "Sum of requests");
    let do_requests = CounterVec::new(do_requests_opts, &["script_name"]).unwrap();
    registry.register(Box::new(do_requests.clone())).unwrap();

    let do_response_body_size_bytes_opts = Opts::new("cloudflare_durable_objects_response_body_size_bytes", "Response body size - bytes");
    let do_response_body_size_bytes = GaugeVec::new(do_response_body_size_bytes_opts, &["script_name", "quantile"]).unwrap();
    registry.register(Box::new(do_response_body_size_bytes.clone())).unwrap();

    let do_wall_time_microseconds_opts = Opts::new("cloudflare_durable_objects_wall_time_microseconds", "Wall time - microseconds");
    let do_wall_time_microseconds = GaugeVec::new(do_wall_time_microseconds_opts, &["script_name", "quantile"]).unwrap();
    registry.register(Box::new(do_wall_time_microseconds.clone())).unwrap();

    let mut last_datetime: Option<Time> = None;
    for account in response_data.clone().viewer.unwrap().accounts.iter() {
        for group in account.durable_objects_invocations_adaptive_groups.iter() {
            let dimensions = group.dimensions.as_ref().unwrap();
            last_datetime = Some(dimensions.datetime_minute.clone());
            let script_name = dimensions.script_name.clone();
            let sum = group.sum.as_ref().unwrap();
            let quantiles = group.quantiles.as_ref().unwrap();

            do_errors.with_label_values(&[script_name.as_str()]).inc_by(sum.errors as f64);
            do_requests.with_label_values(&[script_name.as_str()]).inc_by(sum.requests as f64);

            do_response_body_size_bytes.with_label_values(&[script_name.as_str(), "P25"]).set(quantiles.response_body_size_p25 as f64);
            do_response_body_size_bytes.with_label_values(&[script_name.as_str(), "P50"]).set(quantiles.response_body_size_p50 as f64);
            do_response_body_size_bytes.with_label_values(&[script_name.as_str(), "P75"]).set(quantiles.response_body_size_p75 as f64);
            do_response_body_size_bytes.with_label_values(&[script_name.as_str(), "P90"]).set(quantiles.response_body_size_p90 as f64);
            do_response_body_size_bytes.with_label_values(&[script_name.as_str(), "P99"]).set(quantiles.response_body_size_p99 as f64);
            do_response_body_size_bytes.with_label_values(&[script_name.as_str(), "P999"]).set(quantiles.response_body_size_p999 as f64);

            do_wall_time_microseconds.with_label_values(&[script_name.as_str(), "P25"]).set(quantiles.wall_time_p25 as f64);
            do_wall_time_microseconds.with_label_values(&[script_name.as_str(), "P50"]).set(quantiles.wall_time_p50 as f64);
            do_wall_time_microseconds.with_label_values(&[script_name.as_str(), "P75"]).set(quantiles.wall_time_p75 as f64);
            do_wall_time_microseconds.with_label_values(&[script_name.as_str(), "P90"]).set(quantiles.wall_time_p90 as f64);
            do_wall_time_microseconds.with_label_values(&[script_name.as_str(), "P99"]).set(quantiles.wall_time_p99 as f64);
            do_wall_time_microseconds.with_label_values(&[script_name.as_str(), "P999"]).set(quantiles.wall_time_p999 as f64);
        }
    }

    let timestamp: std::time::SystemTime = last_datetime.map(|datetime| {
        let datetime: NaiveDateTime = NaiveDateTime::parse_from_str(&*datetime, "%+").unwrap();
        datetime.and_utc().into()
    }).unwrap_or_else(|| {
        to_std_systemtime(SystemTime::now())
    });

    Ok(prometheus_registry_to_opentelemetry_metrics(registry, timestamp))
}

fn to_std_systemtime(time: web_time::SystemTime) -> std::time::SystemTime {
    let duration = time.duration_since(web_time::SystemTime::UNIX_EPOCH).unwrap();
    std::time::SystemTime::UNIX_EPOCH + duration
}
