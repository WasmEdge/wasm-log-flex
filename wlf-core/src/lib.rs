mod event;
mod event_hub;

pub use event::{Event, EventMeta, LogLevel, Value};
pub use event_hub::{EventHub, EventHubApi};

#[derive(Debug, PartialEq, Eq)]
pub enum ComponentKind {
    Collector,
    Transformer,
    Dispatcher,
}

pub trait ComponentApi: 'static + Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> ComponentKind;
}
