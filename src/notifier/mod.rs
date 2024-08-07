use crate::config::ConfigTrigger;
use crate::k8s::DynamicObject;
pub use http_notification::HttpNotification;

mod http_notification;

pub struct Notifier {
    http: Vec<HttpNotification>,
}

impl Notifier {
    pub fn new(triggers: ConfigTrigger) -> Self {
        Self {
            http: triggers.http,
        }
    }

    pub fn notify(&self, o: DynamicObject) {
        for endpoint in &self.http {
            endpoint.notify(&o);
        }
    }
}
