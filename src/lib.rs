use opentelemetry_sdk::metrics::data::{ResourceMetrics, ScopeMetrics};
use opentelemetry_sdk::Resource;
use opentelemetry_stdout::MetricsData;
use worker::*;
use worker::wasm_bindgen::JsValue;
use crate::gql::{get_workers_analytics_query, perform_my_query};

mod gql;

#[worker::send]
pub async fn do_fetch(
    url: String,
    data: Option<JsValue>,
) -> Result<Response> {
    let mut init = RequestInit::new();
    init.method = Method::Post;
    init.with_body(data);
    Fetch::Request(Request::new_with_init(url.as_str(), &init)?)
        .send()
        .await
}

#[event(fetch)]
async fn main(_req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let metrics_url = env.var("METRICS_URL")?.to_string();
    let cloudflare_api_url = env.var("CLOUDFLARE_API_URL")?.to_string();
    let cloudflare_api_key = env.var("CLOUDFLARE_API_KEY")?.to_string();

    let result = perform_my_query(cloudflare_api_url, cloudflare_api_key, get_workers_analytics_query::Variables {
        account_tag: Some("123".to_string()),
        datetime_start: Some("2021-01-01T00:00:00Z".to_string()),
        datetime_end: Some("2021-01-02T00:00:00Z".to_string()),
        script_name: None,
    }).await;
    let cf_metrics = match result {
        Ok(metrics) => metrics,
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Response::error(format!("Error: {:?}", e), 500);
        }
    };

    let library = opentelemetry::InstrumentationLibrary::new(
        "my-crate",
        Some(env!("CARGO_PKG_VERSION")),
        Some("https:// opentelemetry. io/ schemas/ 1.17.0"),
        None,
    );
    let scope_metrics = ScopeMetrics {
        scope: library,
        metrics: cf_metrics,
    };
    let mut resource_metrics = ResourceMetrics {
        resource: Resource::default(),
        scope_metrics: vec![scope_metrics],
    };
    let metrics = MetricsData::from(&mut resource_metrics);
    let metrics_json = serde_json::to_string(&metrics).unwrap();
    let response = do_fetch(metrics_url, Some(JsValue::from_str(&metrics_json)).into()).await?;
    return Ok(response);
}
