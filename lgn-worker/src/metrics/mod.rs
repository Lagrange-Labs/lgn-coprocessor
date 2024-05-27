use metrics::{counter, gauge, histogram, SharedString};

pub struct Metrics {}

impl Metrics {
    pub fn new() -> Self {
        Self {}
    }

    pub fn increment_tasks_received(&self, task_type: &str) {
        let task_type = SharedString::from(String::from(task_type));
        counter!("zkmr_worker_tasks_received_total", "task_type" => task_type).increment(1);
    }

    pub fn increment_tasks_processed(&self, task_type: &str) {
        let task_type = SharedString::from(String::from(task_type));
        counter!("zkmr_worker_tasks_processed_total", "task_type" => task_type).increment(1);
    }

    pub fn increment_tasks_failed(&self, task_type: &str) {
        let task_type = SharedString::from(String::from(task_type));
        counter!("zkmr_worker_tasks_failed_total", "task_type" => task_type).increment(1);
    }

    pub fn observe_task_processing_duration(&self, task_type: &str, duration: f64) {
        let task_type = SharedString::from(String::from(task_type));
        histogram!("zkmr_worker_task_processing_duration_seconds", "task_type" => task_type)
            .record(duration);
    }

    pub fn increment_websocket_messages_received(&self, message_type: &str) {
        let message_type = SharedString::from(String::from(message_type));
        counter!("zkmr_worker_websocket_messages_received_total", "message_type" => message_type)
            .increment(1);
    }

    pub fn increment_websocket_messages_sent(&self, message_type: &str) {
        let message_type = SharedString::from(String::from(message_type));
        counter!("zkmr_worker_websocket_messages_sent_total", "message_type" => message_type)
            .increment(1);
    }

    pub fn increment_error_count(&self, error_type: &str) {
        let error_type = SharedString::from(String::from(error_type));
        counter!("zkmr_worker_error_count", "error_type" => error_type).increment(1);
    }

    pub fn increment_gateway_connection_count(&self) {
        gauge!("zkmr_worker_gateway_connection_count").increment(1);
    }
}
