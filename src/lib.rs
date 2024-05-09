use std::env;
use chrono::SubsecRound;
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_sdk::metrics::data::{Metric, ResourceMetrics, ScopeMetrics};
use opentelemetry_sdk::Resource;
use prost::Message;

use worker::*;
use worker::js_sys::Uint8Array;
use worker::wasm_bindgen::JsValue;
use crate::gql::{get_workers_analytics_query, do_get_workers_analytics_query, do_get_d1_analytics_query, get_d1_analytics_query, do_get_durableobjects_analytics_query, get_durable_objects_analytics_query, do_get_queue_backlog_analytics_query, get_queue_backlog_analytics_query};

mod gql;
mod metrics;

#[worker::send]
pub async fn do_fetch(
    url: String,
    headers: String,
    data: Option<JsValue>,
    content_type: String
) -> Result<Response> {
    let mut http_headers = Headers::new();
    // split headers by command, and then by =
    for header in headers.split(",") {
        let parts: Vec<&str> = header.splitn(2, "=").collect();
        if parts.len() == 2 {
            let key = parts[0].trim();
            let value = parts[1].trim();
            http_headers.set(key, value).expect("failed to construct header");
        }
    }
    http_headers.set("Content-Type", &*content_type).expect("failed to construct content-type header");
    let mut init = RequestInit::new();
    init.method = Method::Post;
    init.with_body(data).with_headers(http_headers);
    Fetch::Request(Request::new_with_init(url.as_str(), &init)?)
        .send()
        .await
}

#[event(fetch)]
async fn fetch(_req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let res = do_trigger(env).await;
    match res {
        Ok(_) => Response::ok("OK"),
        Err(_) => Response::error("Error", 500)
    }
}

#[event(scheduled)]
async fn main(_req: ScheduledEvent, env: Env, _ctx: ScheduleContext) -> () {
    let res = do_trigger(env).await;
    match res {
        Ok(_) => console_log!("OK"),
        Err(e) => console_log!("Error: {:?}", e),
    }
}

async fn do_trigger(env: Env) -> Result<()> {
    let cloudflare_api_url = env.var("CLOUDFLARE_API_URL")?.to_string();
    let cloudflare_api_key = env.var("CLOUDFLARE_API_KEY")?.to_string();
    let cloudflare_account_id = env.var("CLOUDFLARE_ACCOUNT_ID")?.to_string();

    let end = chrono::Utc::now().round_subsecs(0);
    let start = (end - chrono::Duration::minutes(1)).round_subsecs(0);

    console_log!("Fetching!");
    let mut all_metrics = Vec::new();

    let result = do_get_workers_analytics_query(&cloudflare_api_url, &cloudflare_api_key, get_workers_analytics_query::Variables {
        account_tag: cloudflare_account_id.clone(),
        datetime_start: Some(start.to_rfc3339()),
        datetime_end: Some(end.to_rfc3339()),
        limit: 9999,
    }).await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        },
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };

    let result = do_get_d1_analytics_query(&cloudflare_api_url, &cloudflare_api_key, get_d1_analytics_query::Variables {
        account_tag: cloudflare_account_id.clone(),
        datetime_start: Some(start.to_rfc3339()),
        datetime_end: Some(end.to_rfc3339()),
        limit: 9999,
    }).await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        },
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };

    let result = do_get_durableobjects_analytics_query(&cloudflare_api_url, &cloudflare_api_key, get_durable_objects_analytics_query::Variables {
        account_tag: cloudflare_account_id.clone(),
        datetime_start: Some(start.to_rfc3339()),
        datetime_end: Some(end.to_rfc3339()),
        limit: 9999,
    }).await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        },
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };

    let result = do_get_queue_backlog_analytics_query(&cloudflare_api_url, &cloudflare_api_key, get_queue_backlog_analytics_query::Variables {
        account_tag: cloudflare_account_id.clone(),
        datetime_start: Some(start.to_rfc3339()),
        datetime_end: Some(end.to_rfc3339()),
        limit: 9999,
    }).await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        },
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };
    console_log!("Done fetching!");

    do_push_metrics(env, all_metrics).await
}

async fn do_push_metrics(env: Env, metrics: Vec<Metric>) -> Result<()> {
    let metrics_url = env.var("METRICS_URL")?.to_string();
    let otlp_headers = match env.var("OTLP_HEADERS") {
        Ok(val) => val.to_string(),
        Err(_) => String::from(""),
    };
    let otlp_encoding_json: bool = match env.var("OTLP_ENCODING") {
        Ok(val) => match val.to_string().to_lowercase().as_str() {
            "json" => true,
            _ => false,
        }
        Err(_) => false,
    };

    console_log!("Converting metrics to OTLP.");
    let library = opentelemetry::InstrumentationLibrary::new(
        "cloudflare-otlp-exporter",
        Some(env!("CARGO_PKG_VERSION")),
        Some("https://github.com/j-white/cloudflare-otlp-exporter/v1.0.0"),
        None,
    );
    let scope_metrics = ScopeMetrics {
        scope: library,
        metrics
    };
    let resource_metrics = ResourceMetrics {
        resource: Resource::empty(),
        scope_metrics: vec![scope_metrics],
    };

    let metrics = ExportMetricsServiceRequest::from(&resource_metrics);
    let js_value: JsValue;
    let content_type: String;
    if otlp_encoding_json {
        let metrics_json = serde_json::to_string(&metrics).unwrap();
        js_value = JsValue::from_str(&metrics_json);
        content_type = "application/json".to_string();
    } else {
        let bytes = metrics.encode_to_vec();
        let array = Uint8Array::from(bytes.as_slice());
        js_value = JsValue::from(array);
        content_type = "application/x-protobuf".to_string();
    }
    console_log!("Done converting metrics to OTLP.");

    console_log!("Posting metrics to OTLP endpoint.");
    let mut res = do_fetch(metrics_url, otlp_headers, Some(js_value).into(), content_type).await?;
    let body = res.text().await?;
    console_log!("Done posting metrics status={} body={:?}", res.status_code(), body);

    if res.status_code() != 200 {
        return Err(Error::JsError(body));
    }
    return Ok(());
}