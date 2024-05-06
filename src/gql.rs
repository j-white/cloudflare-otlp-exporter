use std::borrow::Cow;
use std::error::Error;
use std::sync::{Arc, Mutex};
use graphql_client::{GraphQLQuery, Response};
use opentelemetry::KeyValue;
use opentelemetry::metrics::Unit;
use opentelemetry_sdk::AttributeSet;
use opentelemetry_sdk::metrics::data::{DataPoint, Metric};
use opentelemetry_sdk::metrics::data::Gauge;

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

pub async fn perform_my_query(cloudflare_api_url: String, cloudflare_api_key: String, variables: get_workers_analytics_query::Variables) -> Result<Vec<Metric>, Box<dyn Error>> {
    let metrics = Arc::new(Mutex::new(Vec::new()));
    let request_body = GetWorkersAnalyticsQuery::build_query(variables);
    let client = reqwest::Client::new();
    let res = client.post(cloudflare_api_url)
        .bearer_auth(cloudflare_api_key)
        .json(&request_body).send().await?;
    if !res.status().is_success() {
        return Err(Box::new(res.error_for_status().unwrap_err()));
    }
    let response_body: Response<get_workers_analytics_query::ResponseData> = res.json().await?;

    let response_data: get_workers_analytics_query::ResponseData = response_body.data.expect("missing response data");
    for account in response_data.clone().viewer.unwrap().accounts.iter() {
        for worker in account.workers_invocations_adaptive.iter() {
            // See https://github.com/lablabs/cloudflare-exporter/blob/05e80d9cc5034c5a40b08f7630e6ca5a54c66b20/prometheus.go#L44C61-L44C93
            let requests = worker.sum.as_ref().unwrap().requests;
            let metric = create_metric("cloudflare_worker_requests".to_string(), "A gauge of the number of requests to a worker.".to_string(), requests, "requests".to_string()).unwrap();
            metrics.lock().unwrap().push(metric);
        }
    }

    let mut vec = metrics.lock().unwrap();
    let mut metrics_to_return: Vec<Metric> = Vec::new();
    metrics_to_return.extend(vec.drain(..));
    Ok(metrics_to_return)
}

fn create_metric(name: String, description: String, value: uint64, unit:String) -> Result<Metric, Box<dyn Error>> {
    let key_value = KeyValue::new("key", "value");
    let attribute_set: AttributeSet = std::slice::from_ref(&key_value).into();
    let data_point = DataPoint {
        attributes: attribute_set,
        start_time: None,
        time: None,
        value,
        exemplars: vec![],
    };
    let sample: Gauge<u64> = Gauge {
        data_points: vec![data_point],
    };
    Ok(Metric {
        name: Cow::from(name),
        description: Cow::from(description),
        unit: Unit::new(unit),
        data: Box::new(sample),
    })
}
