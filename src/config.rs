use crate::notifier::{HttpNotification, SqsNotification};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ConfigResource {
    pub group: Option<String>,
    pub api_version: Option<String>,
    pub kind: String,
    pub plural: Option<String>,
    pub label_selector: Option<String>,
    pub field_selector: Option<String>,
    pub namespace: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ConfigTrigger {
    pub http: Option<Vec<HttpNotification>>,
    pub sqs: Option<Vec<SqsNotification>>,
}
