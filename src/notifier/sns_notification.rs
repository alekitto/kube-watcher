use crate::k8s::DynamicObject;
use log::{debug, error};
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Deserialize, Debug)]
pub struct SnsNotification {
    pub(super) topic_arn: String,
}

impl SnsNotification {
    pub(super) fn notify(&self, o: &DynamicObject) {
        let endpoint = self.clone();

        let sms_message = format!(
            "Object {}/{}",
            o.metadata.namespace.as_deref().unwrap_or("default"),
            o.metadata.name.as_deref().unwrap_or("unknown")
        );
        let request_body = json!({
            "source": "io.k8s-watcher",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "object": o,
        })
        .to_string();

        tokio::task::spawn(async move {
            let config = aws_config::load_from_env().await;
            let client = aws_sdk_sns::Client::new(&config);

            match client
                .publish()
                .set_topic_arn(Some(endpoint.topic_arn.clone()))
                .subject("K8s Watcher notification")
                .message(
                    json!({
                        "default": request_body.clone(),
                        "sms": sms_message,
                        "email-json": request_body.clone(),
                        "http": request_body.clone(),
                        "https": request_body.clone(),
                        "sqs": request_body.clone(),
                    })
                    .to_string(),
                )
                .send()
                .await
            {
                Ok(_) => debug!(
                    "aws sns notification sent successfully to {}",
                    &endpoint.topic_arn
                ),
                Err(e) => error!(
                    "error notifying sns endpoint {}: {}",
                    &endpoint.topic_arn,
                    e.to_string()
                ),
            }
        });
    }
}
