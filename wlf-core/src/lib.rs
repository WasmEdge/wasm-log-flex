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
    fn id(&self) -> &str;
    fn kind(&self) -> ComponentKind;
    async fn run(&self, router: Arc<EventRouter>) -> Result<(), Box<dyn Error>>;
}
