use std::sync::Arc;

use eagle_core::{config::SourceDecl, EagleClient, EagleEndpoint, Origin};
use tokio::{runtime::Handle, task::JoinHandle};

pub struct SourceState {
    pub origin: Arc<Origin>,
    pub handle: JoinHandle<()>,
}

pub fn spawn_source(handle: &Handle, decl: SourceDecl, endpoint: EagleEndpoint) -> SourceState {
    let mut source = decl.source;
    let origin = Arc::new(decl.origin);
    let cloned_origin = origin.clone();

    let handle = handle.spawn(async move {
        let client = EagleClient {
            origin: cloned_origin,
            endpoint,
        };

        let instance_id = client.origin.instance_id().to_string();
        tracing::info!(target = instance_id.as_str(), "Source started");
        match source.produce(client).await {
            Ok(_) => {
                tracing::info!(target = instance_id.as_str(), "Source exited");
            }

            Err(e) => {
                tracing::error!(
                    target = instance_id.as_str(),
                    "Source exited with an error: {}",
                    e
                );
            }
        }
    });

    SourceState { origin, handle }
}
