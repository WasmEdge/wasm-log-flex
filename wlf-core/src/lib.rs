mod event;
pub mod event_router;

pub use event::{Event, EventMeta};
pub use serde_json::Value;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ComponentKind {
    Collector,
    Transformer,
    Dispatcher,
}

pub trait ComponentApi: 'static + Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> ComponentKind;
}
