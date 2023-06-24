mod event;
pub mod event_hub;
mod value;

pub use event::{Event, EventMeta};
pub use value::{LogLevel, Value};

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
