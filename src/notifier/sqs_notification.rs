use crate::k8s::DynamicObject;
use log::{debug, error};
use serde::Deserialize;
use serde_json::json;
use url::Url;

#[derive(Clone, Deserialize, Debug)]
pub struct SqsNotification {
    pub(super) url: Url,
}

impl SqsNotification {
    pub(super) fn notify(&self, o: &DynamicObject) {
        let endpoint = self.clone();
        let request_body = json!({
            "source": "io.k8s-watcher",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "object": o,
        })
        .to_string();

        tokio::task::spawn(async move {
            let config = aws_config::load_from_env().await;
            let client = aws_sdk_sqs::Client::new(&config);

            match client
                .send_message()
                .queue_url(endpoint.url.clone())
                .message_body(request_body)
                .send()
                .await
            {
                Ok(_) => debug!(
                    "aws sqs notification sent successfully to {}",
                    &endpoint.url
                ),
                Err(e) => error!(
                    "error notifying sqs endpoint {}: {}",
                    &endpoint.url,
                    e.to_string()
                ),
            }
        });
    }
}
