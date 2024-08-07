use crate::notifier::Notifier;
use crate::watcher::Watcher;
use ::config::{Config, File};
use clap::Parser;
use futures::{stream, FutureExt, TryStreamExt};
use k8s::DynamicObject;
use log::{debug, info};
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::sleep;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};

mod config;
mod k8s;
mod notifier;
mod watcher;

/// Watch for kube resources and send events to remote listeners
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

struct NotifierSubsystem {
    notifier: Notifier,
    receiver: UnboundedReceiver<DynamicObject>,
}

impl NotifierSubsystem {
    async fn run(mut self, subsys: SubsystemHandle) -> Result<(), Box<dyn Error + Send + Sync>> {
        while !self.receiver.is_closed() {
            if let Some(Some(o)) = self.receiver.recv().now_or_never() {
                self.notifier.notify(o);
            }

            if subsys.is_shutdown_requested() {
                break;
            } else {
                sleep(Duration::from_secs(3)).await;
            }
        }

        info!("Shutdown requested, terminating...");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args = Args::parse();
    let config = Config::builder()
        .add_source(File::from(PathBuf::from(args.config)))
        .build()?;

    let watcher = Watcher::new(config.get("resource")?);
    let (watchers, streams) = watcher.watch().await?;

    let notifier = Notifier::new(config.get("trigger")?);
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

    let notifier_subsys = NotifierSubsystem { receiver, notifier };

    Toplevel::new(|s| async move {
        s.start(SubsystemBuilder::new("notifier", |a| {
            notifier_subsys.run(a)
        }));
        for (idx, watch_subsys) in watchers.into_iter().enumerate() {
            s.start(SubsystemBuilder::new(format!("watcher_{}", idx), |a| {
                watch_subsys.run(a)
            }));
        }

        let mut combo_stream = stream::select_all(streams);
        loop {
            select! {
                Ok(Some(o)) = combo_stream.try_next() => {
                    let metadata = o.metadata.clone();

                    debug!(
                        "changes detected for object {}/{}",
                        metadata.namespace.unwrap_or_default(),
                        metadata.name.unwrap_or_default()
                    );

                    sender.send(o).unwrap();
                },
                _ = s.on_shutdown_requested() => break,
                else => continue,
            }
        }

        s.wait_for_children().await;
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_secs(60))
    .await
    .map_err(Into::into)
}
