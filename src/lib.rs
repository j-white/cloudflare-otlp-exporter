use opentelemetry::{global, KeyValue};
use opentelemetry::metrics::{MetricsError, Unit};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use worker::*;

mod gql;

fn init_metrics() ->
                  core::result::Result<SdkMeterProvider, MetricsError> {
    let export_config = opentelemetry_otlp::ExportConfig {
        endpoint: "http://localhost:4318/v1/metrics".to_string(),
        ..opentelemetry_otlp::ExportConfig::default()
    };
    let provider = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::TokioCurrentThread)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_export_config(export_config),
        )
        .build();
    provider
}

fn flush_and_shutdown(provider: SdkMeterProvider) -> core::result::Result<(), MetricsError> {
    provider.force_flush()?;
    provider.shutdown()?;
    Ok(())
}

#[event(fetch)]
async fn main(_req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let maybe_provider = init_metrics();
    let provider = match maybe_provider {
        Ok(provider) => provider,
        Err(err) => {
            console_log!("{}", err.to_string());
            return Response::error("failed to init metrics", 500);
        }
    };

    let variables = gql::get_workers_analytics_query::Variables {
        account_tag: Some("".to_string()),
        datetime_start: Some("".to_string()),
        datetime_end: Some("".to_string()),
        script_name: Some("".to_string()),
    };
    let _ = gql::perform_my_query(variables);

    // Create a meter from the above MeterProvider.
    let meter = global::meter("mylibraryname");
    let counter = meter.u64_counter("my_counter").init();
    counter.add(
        10,
        &[
            KeyValue::new("mykey1", "myvalue1"),
            KeyValue::new("mykey2", "myvalue2"),
        ],
    );

    // Create a Gauge Instrument.
    {
        let gauge = meter
            .f64_gauge("my_gauge")
            .with_description("A gauge set to 1.0")
            .with_unit(Unit::new("myunit"))
            .init();

        gauge.record(
            1.0,
            &[
                KeyValue::new("mykey1", "myvalue1"),
                KeyValue::new("mykey2", "myvalue2"),
            ],
        );
    }

    let res = flush_and_shutdown(provider);
    match res {
        Ok(_) => {
            Response::ok("metrics flushed")
        }
        Err(err) => {
            console_log!("{}", err.to_string());
            Response::error("failed to flushed", 500)
        }
    }
}
