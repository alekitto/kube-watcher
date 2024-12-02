use crate::k8s::DynamicObject;
use log::{debug, error};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use serde::Deserialize;
use serde_json::json;
use url::Url;

#[derive(Clone, Deserialize, Debug)]
pub struct HttpNotification {
    pub(super) url: Url,
    pub(super) retry: Option<usize>,
}

impl HttpNotification {
    pub(super) fn notify(&self, o: &DynamicObject) {
        let request_body = json!({
            "source": "io.k8s-watcher",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "object": o,
        })
        .to_string();

        let endpoint = self.clone();

        tokio::task::spawn(async move {
            let retry_policy = ExponentialBackoff::builder()
                .base(10)
                .build_with_max_retries(endpoint.retry.unwrap_or(3) as u32);
            let client = ClientBuilder::new(reqwest::Client::new())
                .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                .build();

            match client
                .post(endpoint.url.clone())
                .header("User-Agent", "rs-kubewatcher/1.0")
                .header("Content-Type", "application/json; charset=utf8")
                .body(request_body)
                .send()
                .await
            {
                Ok(_) => debug!("http notification sent successfully to {}", &endpoint.url),
                Err(e) => error!(
                    "error notifying http endpoint {}: {}",
                    &endpoint.url,
                    e.to_string()
                ),
            }
        });
    }
}
