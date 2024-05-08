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
query_path = "gql/queries.graphql",
response_derives = "Debug,Clone"
)]
pub struct GetWorkersAnalyticsQuery;

#[allow(non_camel_case_types)]
type float32 = f32;

#[allow(non_camel_case_types)]
type string = String;

#[allow(non_camel_case_types)]
type Time = String;

#[allow(non_camel_case_types)]
type uint64 = u64;

#[allow(non_camel_case_types)]
type float64 = f64;

pub async fn perform_my_query(cloudflare_api_url: String, cloudflare_api_key: String, variables: get_workers_analytics_query::Variables) -> Result<Vec<Metric>, Box<dyn Error>> {
    let request_body = GetWorkersAnalyticsQuery::build_query(variables);
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
    let worker_requests_opts = Opts::new("cloudflare_worker_requests", "number of requests to the worker");
    let worker_requests = CounterVec::new(worker_requests_opts, &["script_name"]).unwrap();
    registry.register(Box::new(worker_requests.clone())).unwrap();

    let worker_errors_opts = Opts::new("cloudflare_worker_errors", "number of failed requests to the worker");
    let worker_errors = CounterVec::new(worker_errors_opts, &["script_name"]).unwrap();
    registry.register(Box::new(worker_errors.clone())).unwrap();

    let worker_cpu_time_opts = Opts::new("cloudflare_worker_cpu_time", "cpu time processing request");
    let worker_cpu_time = GaugeVec::new(worker_cpu_time_opts, &["script_name", "quantile"]).unwrap();
    registry.register(Box::new(worker_cpu_time.clone())).unwrap();

    let worker_duration_opts = Opts::new("cloudflare_worker_duration", "wall clock time processing request");
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

fn to_std_systemtime(time: web_time::SystemTime) -> std::time::SystemTime {
    let duration = time.duration_since(web_time::SystemTime::UNIX_EPOCH).unwrap();
    std::time::SystemTime::UNIX_EPOCH + duration
}
