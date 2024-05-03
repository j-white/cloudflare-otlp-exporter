use std::error::Error;
use graphql_client::GraphQLQuery;
use opentelemetry::{global, KeyValue};
use opentelemetry::metrics::Unit;

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

pub async fn perform_my_query(variables: get_workers_analytics_query::Variables) -> Result<(), Box<dyn Error>> {
    let request_body = GetWorkersAnalyticsQuery::build_query(variables);
    let client = reqwest::Client::new();
    let res = client.post("/graphql").json(&request_body).send().await?;
    let response: get_workers_analytics_query::ResponseData = res.json().await?;
    let _ = response.viewer.unwrap().accounts.iter().map(|account| account.workers_invocations_adaptive.iter().map(|worker| {
        // See https://github.com/lablabs/cloudflare-exporter/blob/05e80d9cc5034c5a40b08f7630e6ca5a54c66b20/prometheus.go#L44C61-L44C93
        let requests = worker.sum.as_ref().unwrap().requests;
        let meter = global::meter("cloudflare_worker_requests");
        let gauge = meter
            .u64_gauge("count")
            .with_description("A gauge of the number of requests to a worker.")
            .with_unit(Unit::new("requests"))
            .init();
        gauge.record(
            requests,
            &[
                KeyValue::new("script_name", worker.dimensions.as_ref().unwrap().script_name.clone())
            ],
        );
    }));
    Ok(())
}
