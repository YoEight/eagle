use std::sync::Arc;

use eagle_core::{Origin, Source, EagleClient, EagleEndpoint};
use tokio::task::JoinHandle;

pub struct SourceState {
    origin: Arc<Origin>,
    handle: JoinHandle<()>,
}

pub fn spawn_source<S>(origin: Origin, endpoint: EagleEndpoint, source: S) -> SourceState
where
    S: Source + Send + 'static,
{
    let origin = Arc::new(origin);
    let cloned_origin = origin.clone();

    let handle = tokio::spawn(async move {
        let client = EagleClient {
            origin: cloned_origin,
            endpoint,
        };

        source.produce(client).await;
    });

    SourceState {
        origin,
        handle,
    }
}
