use opentelemetry_sdk::metrics::data::{ResourceMetrics, ScopeMetrics};
use opentelemetry_sdk::Resource;
use opentelemetry_stdout::MetricsData;
use worker::*;
use worker::wasm_bindgen::JsValue;
use crate::gql::{get_workers_analytics_query, perform_my_query};

mod gql;
mod metrics;

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
    let cloudflare_account_id = env.var("CLOUDFLARE_ACCOUNT_ID")?.to_string();

    let end = chrono::Utc::now();
    let start = end - chrono::Duration::minutes(1);

    let result = perform_my_query(cloudflare_api_url, cloudflare_api_key, get_workers_analytics_query::Variables {
        account_tag: Some(cloudflare_account_id),
        datetime_start: Some(start.to_string()),
        datetime_end: Some(end.to_string()),
        limit: 9999,
    }).await;
    let cf_metrics = match result {
        Ok(metrics) => metrics,
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Response::error(format!("Error: {:?}", e), 500);
        }
    };

    let library = opentelemetry::InstrumentationLibrary::new(
        "cloudflare-otlp-exporter",
        Some(env!("CARGO_PKG_VERSION")),
        Some("https://github.com/j-white/cloudflare-otlp-exporter/v1.0.0"),
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
