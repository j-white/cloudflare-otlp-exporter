use std::error::Error;
use graphql_client::GraphQLQuery;

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
        return worker.sum.as_ref().unwrap().subrequests;
    }));
    Ok(())
}
