use crate::config::ConfigTrigger;
use crate::k8s::DynamicObject;
pub use http_notification::HttpNotification;
pub use sqs_notification::SqsNotification;

mod http_notification;
mod sqs_notification;

pub struct Notifier {
    http: Vec<HttpNotification>,
    sqs: Vec<SqsNotification>,
}

impl Notifier {
    pub fn new(triggers: ConfigTrigger) -> Self {
        Self {
            http: triggers.http.unwrap_or_default(),
            sqs: triggers.sqs.unwrap_or_default(),
        }
    }

    pub fn notify(&self, o: DynamicObject) {
        for endpoint in &self.sqs {
            endpoint.notify(&o);
        }

        for endpoint in &self.http {
            endpoint.notify(&o);
        }
    }
}
