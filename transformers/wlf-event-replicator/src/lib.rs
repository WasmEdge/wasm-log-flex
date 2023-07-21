use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use thiserror::Error;
use tracing::info;
use wlf_core::{
    event_router::{EventRouter, EventRouterApi},
    ComponentApi, ComponentKind,
};

#[derive(Deserialize, Debug)]
pub struct EventReplicator {
    id: String,
    destinations: Vec<String>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("event router error, {0}")]
    EventRouter(#[from] wlf_core::event_router::Error),
}

#[async_trait]
impl ComponentApi for EventReplicator {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Transformer
    }

    async fn run(&self, router: Arc<EventRouter>) -> Result<(), Box<dyn std::error::Error>> {
        while let Ok(event) = router.poll_event(self.id()).await {
            info!("{} receives new event:\n\t{event:?}", self.id);

            for d in &self.destinations {
                router.send_event(event.clone(), d).await?;
            }
        }
        Ok(())
    }
}
