use opentelemetry_proto::tonic::common::v1::{any_value, AnyValue, KeyValue};
use opentelemetry_proto::tonic::metrics::v1::{
    metric, number_data_point, AggregationTemporality, Gauge, Metric, NumberDataPoint, Sum,
};
use prometheus::proto::{LabelPair, MetricFamily};
use prometheus::Registry;

pub fn prometheus_registry_to_opentelemetry_metrics(
    registry: Registry,
    timestamp_nanos: u64,
) -> Vec<Metric> {
    let mut vec = Vec::new();
    for metric_family in registry.gather() {
        vec.push(create_metric_prom(&metric_family, timestamp_nanos));
    }
    vec
}

fn to_attributes(labels: &[LabelPair]) -> Vec<KeyValue> {
    let mut attributes = Vec::new();
    for label in labels {
        attributes.push(KeyValue {
            key: label.name().to_string(),
            value: Some(AnyValue {
                value: Some(any_value::Value::StringValue(label.value().to_string())),
            }),
        });
    }
    attributes
}

fn get_otlp_name_and_unit_from_prom_name(name: &str) -> (String, String) {
    let (otlp_name, unit) = name.rsplit_once('_').unwrap();
    (otlp_name.to_string(), unit.to_string())
}

fn create_metric_prom(metric_family: &MetricFamily, timestamp_nanos: u64) -> Metric {
    let is_counter = metric_family
        .get_metric()
        .first()
        .map(|metric| metric.get_counter().is_some())
        .unwrap_or(false);

    if is_counter {
        let mut data_points = Vec::new();
        for metric in metric_family.get_metric() {
            let counter = metric.get_counter();
            let value = counter.as_ref().and_then(|c| c.value).unwrap_or(0.0);
            let data_point = NumberDataPoint {
                attributes: to_attributes(metric.get_label()),
                start_time_unix_nano: timestamp_nanos,
                time_unix_nano: timestamp_nanos,
                value: Some(number_data_point::Value::AsDouble(value)),
                exemplars: vec![],
                flags: 0,
            };
            data_points.push(data_point);
        }
        let sum = Sum {
            data_points,
            aggregation_temporality: AggregationTemporality::Cumulative as i32,
            // See https://opentelemetry.io/docs/specs/otel/compatibility/prometheus_and_openmetrics/#otlp-metric-points-to-prometheus
            // if the metric is monotonic, then "_total" gets appended to name
            is_monotonic: false,
        };
        let (name, unit) = get_otlp_name_and_unit_from_prom_name(metric_family.name());
        Metric {
            name,
            description: metric_family.help().to_owned(),
            unit,
            metadata: vec![],
            data: Some(metric::Data::Sum(sum)),
        }
    } else {
        let mut data_points = Vec::new();
        for metric in metric_family.get_metric() {
            let gauge = metric.get_gauge();
            let value = gauge.as_ref().and_then(|g| g.value).unwrap_or(0.0);
            let data_point = NumberDataPoint {
                attributes: to_attributes(metric.get_label()),
                start_time_unix_nano: timestamp_nanos,
                time_unix_nano: timestamp_nanos,
                value: Some(number_data_point::Value::AsDouble(value)),
                exemplars: vec![],
                flags: 0,
            };
            data_points.push(data_point);
        }
        let gauge = Gauge { data_points };
        let (name, unit) = get_otlp_name_and_unit_from_prom_name(metric_family.name());
        Metric {
            name,
            description: metric_family.help().to_owned(),
            unit,
            metadata: vec![],
            data: Some(metric::Data::Gauge(gauge)),
        }
    }
}
