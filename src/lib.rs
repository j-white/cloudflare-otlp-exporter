use chrono::SubsecRound;
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_proto::tonic::common::v1::InstrumentationScope;
use opentelemetry_proto::tonic::metrics::v1::{Metric, ResourceMetrics, ScopeMetrics};
use prost::Message;

use crate::gql::{
    do_get_d1_analytics_query, do_get_durableobjects_analytics_query,
    do_get_queue_backlog_analytics_query, do_get_queue_operations_analytics_query,
    do_get_workers_analytics_query, do_get_zone_http_requests_by_colo_query,
    do_get_zone_http_requests_query, get_d1_analytics_query,
    get_durable_objects_analytics_query, get_queue_backlog_analytics_query,
    get_queue_operations_analytics_query, get_workers_analytics_query,
    get_zone_http_requests_by_colo_query, get_zone_http_requests_query,
};
use worker::js_sys::Uint8Array;
use worker::wasm_bindgen::JsValue;
use worker::*;

mod gql;
mod metrics;

const DEFAULT_SCRAPE_DELAY_SECONDS: i64 = 300;

fn get_scrape_delay_seconds(env: &Env) -> i64 {
    env.var("SCRAPE_DELAY")
        .ok()
        .and_then(|val| val.to_string().parse::<i64>().ok())
        .filter(|val| *val >= 0)
        .unwrap_or(DEFAULT_SCRAPE_DELAY_SECONDS)
}

#[worker::send]
pub async fn do_fetch(
    url: String,
    headers: String,
    data: Option<JsValue>,
    content_type: String,
) -> Result<Response> {
    let http_headers = Headers::new();
    // split headers by command, and then by =
    for header in headers.split(",") {
        let parts: Vec<&str> = header.splitn(2, "=").collect();
        if parts.len() == 2 {
            let key = parts[0].trim();
            let value = parts[1].trim();
            http_headers
                .set(key, value)
                .expect("failed to construct header");
        }
    }
    http_headers
        .set("Content-Type", &content_type)
        .expect("failed to construct content-type header");
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
        Err(_) => Response::error("Error", 500),
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
    let debug_logging: bool = match env.var("DEBUG_LOGGING") {
        Ok(val) => matches!(
            val.to_string().to_lowercase().as_str(),
            "true" | "1" | "yes"
        ),
        Err(_) => false,
    };

    let scrape_delay_seconds = get_scrape_delay_seconds(&env);
    let end =
        (chrono::Utc::now() - chrono::Duration::seconds(scrape_delay_seconds)).round_subsecs(0);
    let start = (end - chrono::Duration::minutes(1)).round_subsecs(0);
    let fallback_timestamp_nanos = end.timestamp_nanos_opt().unwrap_or(0) as u64;

    console_log!("Fetching!");
    let mut all_metrics = Vec::new();

    let result = do_get_workers_analytics_query(
        &cloudflare_api_url,
        &cloudflare_api_key,
        get_workers_analytics_query::Variables {
            account_tag: cloudflare_account_id.clone(),
            datetime_start: start.to_rfc3339(),
            datetime_end: end.to_rfc3339(),
            limit: 9999,
        },
        debug_logging,
        fallback_timestamp_nanos,
    )
    .await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        }
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };

    let result = do_get_d1_analytics_query(
        &cloudflare_api_url,
        &cloudflare_api_key,
        get_d1_analytics_query::Variables {
            account_tag: cloudflare_account_id.clone(),
            datetime_start: start.to_rfc3339(),
            datetime_end: end.to_rfc3339(),
            limit: 9999,
        },
        debug_logging,
        fallback_timestamp_nanos,
    )
    .await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        }
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };

    let result = do_get_durableobjects_analytics_query(
        &cloudflare_api_url,
        &cloudflare_api_key,
        get_durable_objects_analytics_query::Variables {
            account_tag: cloudflare_account_id.clone(),
            datetime_start: start.to_rfc3339(),
            datetime_end: end.to_rfc3339(),
            limit: 9999,
        },
        debug_logging,
        fallback_timestamp_nanos,
    )
    .await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        }
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };

    let result = do_get_queue_backlog_analytics_query(
        &cloudflare_api_url,
        &cloudflare_api_key,
        get_queue_backlog_analytics_query::Variables {
            account_tag: cloudflare_account_id.clone(),
            datetime_start: start.to_rfc3339(),
            datetime_end: end.to_rfc3339(),
            limit: 9999,
        },
        debug_logging,
        fallback_timestamp_nanos,
    )
    .await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        }
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };

    let result = do_get_queue_operations_analytics_query(
        &cloudflare_api_url,
        &cloudflare_api_key,
        get_queue_operations_analytics_query::Variables {
            account_tag: cloudflare_account_id.clone(),
            datetime_start: start.to_rfc3339(),
            datetime_end: end.to_rfc3339(),
            limit: 9999,
        },
        debug_logging,
        fallback_timestamp_nanos,
    )
    .await;
    match result {
        Ok(metrics) => {
            for metric in metrics {
                all_metrics.push(metric);
            }
        }
        Err(e) => {
            console_log!("Querying Cloudflare API failed: {:?}", e);
            return Err(Error::JsError(e.to_string()));
        }
    };

    // Zone HTTP requests metrics (optional - only if CLOUDFLARE_ZONE_IDS is set)
    if let Ok(zone_ids_var) = env.var("CLOUDFLARE_ZONE_IDS") {
        let zone_ids: Vec<String> = zone_ids_var
            .to_string()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if !zone_ids.is_empty() {
            let result = do_get_zone_http_requests_query(
                &cloudflare_api_url,
                &cloudflare_api_key,
                get_zone_http_requests_query::Variables {
                    zone_i_ds: Some(zone_ids.clone()),
                    datetime_start: start.to_rfc3339(),
                    datetime_end: end.to_rfc3339(),
                    limit: 9999,
                },
                debug_logging,
                fallback_timestamp_nanos,
            )
            .await;
            match result {
                Ok(metrics) => {
                    for metric in metrics {
                        all_metrics.push(metric);
                    }
                }
                Err(e) => {
                    console_log!(
                        "Querying Cloudflare API for zone HTTP requests failed: {:?}",
                        e
                    );
                    return Err(Error::JsError(e.to_string()));
                }
            };

            // Optionally query with coloCode dimension for configured hosts
            if let Ok(zone_colo_hosts_var) = env.var("ZONE_COLO_HOSTS") {
                let colo_hosts: std::collections::HashSet<String> = zone_colo_hosts_var
                    .to_string()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !colo_hosts.is_empty() {
                    let result = do_get_zone_http_requests_by_colo_query(
                        &cloudflare_api_url,
                        &cloudflare_api_key,
                        get_zone_http_requests_by_colo_query::Variables {
                            zone_i_ds: Some(zone_ids),
                            datetime_start: start.to_rfc3339(),
                            datetime_end: end.to_rfc3339(),
                            limit: 9999,
                        },
                        debug_logging,
                        fallback_timestamp_nanos,
                        &colo_hosts,
                    )
                    .await;
                    match result {
                        Ok(metrics) => {
                            for metric in metrics {
                                all_metrics.push(metric);
                            }
                        }
                        Err(e) => {
                            console_log!(
                                "Querying Cloudflare API for zone HTTP requests by colo failed: {:?}",
                                e
                            );
                            return Err(Error::JsError(e.to_string()));
                        }
                    };
                }
            }
        }
    }

    console_log!("Done fetching!");

    do_push_metrics(env, all_metrics, debug_logging).await
}

async fn do_push_metrics(env: Env, metrics: Vec<Metric>, debug_logging: bool) -> Result<()> {
    let metrics_url = env.var("METRICS_URL")?.to_string();
    let otlp_headers = match env.var("OTLP_HEADERS") {
        Ok(val) => val.to_string(),
        Err(_) => String::from(""),
    };
    let otlp_encoding_json: bool = match env.var("OTLP_ENCODING") {
        Ok(val) => matches!(val.to_string().to_lowercase().as_str(), "json"),
        Err(_) => false,
    };

    console_log!("Converting metrics to OTLP.");
    let scope = InstrumentationScope {
        name: "cloudflare-otlp-exporter".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        attributes: vec![],
        dropped_attributes_count: 0,
    };
    let scope_metrics = ScopeMetrics {
        scope: Some(scope),
        metrics,
        schema_url: "https://github.com/j-white/cloudflare-otlp-exporter/v1.0.0".to_string(),
    };
    let resource_metrics = ResourceMetrics {
        resource: None,
        scope_metrics: vec![scope_metrics],
        schema_url: String::new(),
    };

    let export_request = ExportMetricsServiceRequest {
        resource_metrics: vec![resource_metrics],
    };

    // Log the OTLP payload as JSON for debugging
    if debug_logging {
        let metrics_json_for_logging = serde_json::to_string_pretty(&export_request).unwrap();
        console_log!("OTLP metrics payload:\n{}", metrics_json_for_logging);
    }

    let js_value: JsValue;
    let content_type: String;
    if otlp_encoding_json {
        let metrics_json = serde_json::to_string(&export_request).unwrap();
        js_value = JsValue::from_str(&metrics_json);
        content_type = "application/json".to_string();
    } else {
        let bytes = export_request.encode_to_vec();
        let array = Uint8Array::from(bytes.as_slice());
        js_value = JsValue::from(array);
        content_type = "application/x-protobuf".to_string();
    }
    console_log!("Done converting metrics to OTLP.");

    console_log!("Posting metrics to OTLP endpoint.");
    let mut res = do_fetch(metrics_url, otlp_headers, Some(js_value), content_type).await?;
    let body = res.text().await?;
    console_log!(
        "Done posting metrics status={} body={:?}",
        res.status_code(),
        body
    );

    if res.status_code() != 200 {
        return Err(Error::JsError(body));
    }
    Ok(())
}
