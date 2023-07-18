//! Utilities for testing

use std::sync::Arc;

use async_trait::async_trait;
use wlf_core::{event_router::EventRouter, ComponentApi, ComponentKind};

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

#[async_trait]
impl ComponentApi for DummyComponent {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> ComponentKind {
        self.kind
    }

    async fn run(&self, _router: Arc<EventRouter>) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
