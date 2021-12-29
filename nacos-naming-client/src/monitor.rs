use prometheus::{
    IntGaugeVec, HistogramVec, Histogram, HistogramTimer, 
    core::{GenericGauge, AtomicI64}
};

use lazy_static::lazy_static;
use prometheus::{
    register_int_gauge_vec, register_histogram_vec
};

lazy_static! {
    static ref NACOS_MONITOR: IntGaugeVec = register_int_gauge_vec!(
        "nacos_monitor", "nacos_monitor", &["module", "name"]).unwrap();
    static ref NACOS_CLIENT_REQUEST_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "nacos_client_request", 
        "nacos_client_request", 
        &["module", "method", "url", "code"]
    ).unwrap();
}

pub fn service_info_map_size_monitor() -> GenericGauge<AtomicI64> {
    NACOS_MONITOR.with_label_values(&["naming", "serviceInfoMapSize"])
}

pub fn dom2beat_size_monitor() -> GenericGauge<AtomicI64> {
    NACOS_MONITOR.with_label_values(&["naming", "dom2BeatSize"])
}

pub fn listen_config_count_monitor() -> GenericGauge<AtomicI64> {
    NACOS_MONITOR.with_label_values(&["naming", "listenConfigCount"])
}

pub fn config_request_monitor(method: &str, url: &str, code: &str) -> HistogramTimer {
    NACOS_CLIENT_REQUEST_HISTOGRAM.with_label_values(&["config", method, url, code]).start_timer()
}

pub fn naming_request_monitor(method: &str, url: &str, code: &str) -> Histogram {
    NACOS_CLIENT_REQUEST_HISTOGRAM.with_label_values(&["config", method, url, code])
}