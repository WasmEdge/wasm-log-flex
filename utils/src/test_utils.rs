//! Utilities for testing

use wlf_core::{ComponentApi, ComponentKind};

pub struct DummyComponent {
    id: String,
    kind: ComponentKind,
}

impl DummyComponent {
    pub fn new(id: impl Into<String>, kind: ComponentKind) -> Self {
        Self {
            id: id.into(),
            kind,
        }
    }
}

impl ComponentApi for DummyComponent {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> ComponentKind {
        self.kind
    }
}
