use crate::config::ConfigResource;
use crate::k8s::{ApiResource, DynamicObject};
use futures::stream::BoxStream;
use futures::{Stream, StreamExt};
use kube::api::TypeMeta;
use kube::runtime::reflector::Store;
use kube::runtime::{reflector, watcher, WatchStreamExt};
use kube::{Api, Client, ResourceExt};
use log::{info, trace};
use std::error::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_graceful_shutdown::SubsystemHandle;

pub struct Watcher {
    resources: Vec<ConfigResource>,
}

pub struct WatchSubsystem {
    reader: Store<DynamicObject>,
}

impl WatchSubsystem {
    pub async fn run(self, subsys: SubsystemHandle) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.reader.wait_until_ready().await.unwrap();
        while !subsys.is_shutdown_requested() {
            let resources = self
                .reader
                .state()
                .iter()
                .map(|r| r.name_any())
                .collect::<Vec<_>>();
            trace!("Current {} resources: {:?}", resources.len(), resources);

            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }

        info!("Shutdown requested, terminating...");
        Ok(())
    }
}

impl Watcher {
    pub fn new(resources: Vec<ConfigResource>) -> Self {
        Self { resources }
    }

    pub async fn watch<'a>(
        self,
    ) -> Result<
        (
            Vec<WatchSubsystem>,
            Vec<impl Stream<Item = Result<DynamicObject, watcher::Error>>>,
        ),
        Box<dyn Error>,
    > {
        let client = Client::try_default().await?;
        let mut streams = vec![];
        let mut subsys = vec![];

        for resource in &self.resources {
            let group = resource.group.clone().unwrap_or_else(|| "".to_string());
            let version = resource
                .api_version
                .clone()
                .unwrap_or_else(|| "v1".to_string());
            let api_version = if group.is_empty() {
                version.clone()
            } else {
                format!("{}/{}", group, version)
            };
            let kind = resource.kind.clone();
            let plural = resource
                .plural
                .clone()
                .unwrap_or_else(|| format!("{}s", kind.to_ascii_lowercase()));

            let api_resource = ApiResource {
                group,
                version,
                api_version,
                kind,
                plural,
            };

            let api: Api<DynamicObject> = if let Some(ns) = &resource.namespace {
                Api::namespaced_with(client.clone(), ns.as_str(), &api_resource)
            } else {
                Api::all_with(client.clone(), &api_resource)
            };

            let (reader, writer) = reflector::store();

            let config = watcher::Config {
                label_selector: resource.label_selector.clone(),
                field_selector: resource.field_selector.clone(),
                ..Default::default()
            };

            let stream = watcher(api, config)
                .default_backoff()
                .reflect(writer)
                .applied_objects()
                .boxed();

            subsys.push(WatchSubsystem { reader });
            streams.push(StreamWrapper {
                api_resource,
                inner: stream,
            });
        }

        Ok((subsys, streams))
    }
}

struct StreamWrapper<'w> {
    api_resource: ApiResource,
    inner: BoxStream<'w, Result<DynamicObject, watcher::Error>>,
}

impl Stream for StreamWrapper<'_> {
    type Item = Result<DynamicObject, watcher::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(mut obj))) => {
                if obj.types.is_none() {
                    obj.types = Some(TypeMeta {
                        api_version: self.api_resource.api_version.clone(),
                        kind: self.api_resource.kind.clone(),
                    })
                }

                Poll::Ready(Some(Ok(obj)))
            }
            res => res,
        }
    }
}
