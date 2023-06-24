use crate::Value;

#[derive(Debug, Clone)]
pub struct Event {
    pub value: Value,
    // usually used by internal components, for tracing, tracking, etc.
    pub meta: EventMeta,
}

#[derive(Debug, Clone)]
pub struct EventMeta {}
