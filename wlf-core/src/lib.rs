mod event;
pub mod event_router;

use std::{error::Error, sync::Arc};

use async_trait::async_trait;
pub use event::{Event, EventMeta};
use event_router::EventRouter;
pub use serde_json::json as value;
pub use serde_json::Value;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ComponentKind {
    Collector,
    Transformer,
    Dispatcher,
}

#[async_trait]
pub trait ComponentApi: 'static + Send + Sync {
    // Returns the unique id of the component.
    fn id(&self) -> &str;
    // Return the component kind(collector, transformer, or dispatcher)
    fn kind(&self) -> ComponentKind;
    // Run the component. Use the `router` to recv/send events from/to other components
    async fn run(&self, router: Arc<EventRouter>) -> Result<(), Box<dyn Error>>;
}
