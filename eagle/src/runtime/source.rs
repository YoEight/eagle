use std::sync::Arc;

use eagle_core::{config::SourceDecl, EagleClient, EagleEndpoint, Origin};
use tokio::task::JoinHandle;

pub struct SourceState {
    pub origin: Arc<Origin>,
    pub handle: JoinHandle<()>,
}

pub fn spawn_source(decl: SourceDecl, endpoint: EagleEndpoint) -> SourceState {
    let mut source = decl.source;
    let origin = Arc::new(decl.origin);
    let cloned_origin = origin.clone();

    let handle = tokio::spawn(async move {
        let client = EagleClient {
            origin: cloned_origin,
            endpoint,
        };

        source.produce(client).await;
    });

    SourceState { origin, handle }
}
