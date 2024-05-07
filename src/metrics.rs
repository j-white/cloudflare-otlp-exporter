use std::borrow::Cow;
use std::time::SystemTime;
use opentelemetry::KeyValue;
use opentelemetry::metrics::Unit;
use opentelemetry_sdk::metrics::data::{DataPoint, Metric, Temporality};
use prometheus::proto::MetricFamily;
use prometheus::Registry;

pub fn prometheus_registry_to_opentelemetry_metrics(registry: Registry, timestamp: SystemTime) -> Vec<Metric> {
    let mut vec = Vec::new();
    for metric_family in registry.gather() {
        for metric in metric_family.get_metric() {
            vec.push(create_metric_prom(&metric_family, metric, timestamp));
        }
    }
    return vec;
}

fn create_metric_prom(metric_family: &MetricFamily, metric: &prometheus::proto::Metric, timestamp: SystemTime) -> Metric {
    // convert metric labels to key value pairs
    let mut labels = Vec::new();
    for label_pair in metric.get_label() {
        let key_value = KeyValue::new(label_pair.get_name().to_owned(), label_pair.get_value().to_owned());
        labels.push(key_value);
    }

    let otlp_metric: Metric;
    if metric.has_gauge() {
        let gauge = metric.get_gauge();
        let data_point = DataPoint {
            attributes: labels.as_slice().into(),
            start_time: None,
            time: Some(timestamp),
            value: gauge.get_value(),
            exemplars: vec![],
        };
        let sample: opentelemetry_sdk::metrics::data::Gauge<f64> = opentelemetry_sdk::metrics::data::Gauge {
            data_points: vec![data_point],
        };
        otlp_metric = Metric {
            name: Cow::from(metric_family.get_name().to_owned()),
            description: Cow::from(metric_family.get_help().to_owned()),
            unit: Unit::new(""),
            data: Box::new(sample),
        }
    } else if metric.has_counter() {
        let counter = metric.get_counter();
        let data_point = DataPoint {
            attributes: labels.as_slice().into(),
            start_time: None,
            time: Some(timestamp),
            value: counter.get_value(),
            exemplars: vec![],
        };
        let sample: opentelemetry_sdk::metrics::data::Sum<f64> = opentelemetry_sdk::metrics::data::Sum {
            data_points: vec![data_point],
            temporality: Temporality::Cumulative,
            is_monotonic: true
        };
        otlp_metric = Metric {
            name: Cow::from(metric_family.get_name().to_owned()),
            description: Cow::from(metric_family.get_help().to_owned()),
            unit: Unit::new(""),
            data: Box::new(sample),
        }
    } else {
        panic!("Unsupported metric type")
    }
    otlp_metric
}
