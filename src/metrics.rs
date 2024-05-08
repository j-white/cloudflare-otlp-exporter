use std::borrow::Cow;
use std::time::SystemTime;
use opentelemetry::KeyValue;
use opentelemetry::metrics::Unit;
use opentelemetry_sdk::AttributeSet;
use opentelemetry_sdk::metrics::data::{DataPoint, Metric, Temporality};
use prometheus::proto::{LabelPair, MetricFamily};
use prometheus::Registry;

pub fn prometheus_registry_to_opentelemetry_metrics(registry: Registry, timestamp: SystemTime) -> Vec<Metric> {
    let mut vec = Vec::new();
    for metric_family in registry.gather() {
        vec.push(create_metric_prom(&metric_family, timestamp));
    }
    return vec;
}

fn to_attributes(labels: &[LabelPair]) -> AttributeSet {
    let mut attributes = Vec::new();
    for label in labels {
        attributes.push(KeyValue::new(label.get_name().to_string(), label.get_value().to_string()));
    }
    attributes.as_slice().into()
}

fn get_otlp_name_and_unit_from_prom_name(name: &str) -> (String, String) {
    let mut parts = name.rsplitn(2, '_');
    let unit = parts.next().unwrap();
    let otlp_name = parts.next().unwrap();
    (otlp_name.to_string(), unit.to_string())
}

fn create_metric_prom(metric_family: &MetricFamily, timestamp: SystemTime) -> Metric {
    let is_counter = metric_family.get_metric().get(0).map(|metric| metric.has_counter()).unwrap_or(false);
    let otlp_metric: Metric;
    if is_counter {
        let mut data_points = Vec::new();
        for metric in metric_family.get_metric() {
            let counter = metric.get_counter();
            let data_point = DataPoint {
                attributes: to_attributes(metric.get_label()),
                start_time: Some(timestamp),
                time: Some(timestamp),
                value: counter.get_value(),
                exemplars: vec![],
            };
            data_points.push(data_point);
        }
        let sample: opentelemetry_sdk::metrics::data::Sum<f64> = opentelemetry_sdk::metrics::data::Sum {
            data_points,
            temporality: Temporality::Cumulative,
            // See https://opentelemetry.io/docs/specs/otel/compatibility/prometheus_and_openmetrics/#otlp-metric-points-to-prometheus
            // if the metric is monotonic, then "_total" gets appended to name
            is_monotonic: false
        };
        let (name, unit) = get_otlp_name_and_unit_from_prom_name(metric_family.get_name());
        otlp_metric = Metric {
            name: Cow::from(name.to_owned()),
            description: Cow::from(metric_family.get_help().to_owned()),
            unit: Unit::new(unit),
            data: Box::new(sample),
        }
    } else {
        let mut data_points = Vec::new();
        for metric in metric_family.get_metric() {
            let gauge = metric.get_gauge();
            let data_point = DataPoint {
                attributes: to_attributes(metric.get_label()),
                start_time: Some(timestamp),
                time: Some(timestamp),
                value: gauge.get_value(),
                exemplars: vec![],
            };
            data_points.push(data_point);
        }
        let sample: opentelemetry_sdk::metrics::data::Gauge<f64> = opentelemetry_sdk::metrics::data::Gauge {
            data_points
        };
        let (name, unit) = get_otlp_name_and_unit_from_prom_name(metric_family.get_name());
        otlp_metric = Metric {
            name: Cow::from(name.to_owned()),
            description: Cow::from(metric_family.get_help().to_owned()),
            unit: Unit::new(unit),
            data: Box::new(sample),
        }
    }
    otlp_metric
}
