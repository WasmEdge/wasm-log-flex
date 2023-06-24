use std::sync::Arc;

use thiserror::Error;
use tracing::info;
use wlf_core::{
    event_hub::{EventHub, EventHubApi},
    ComponentApi, ComponentKind, Value,
};

pub struct BinlogFilter {
    id: String,
    destination: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("event hub error, {0}")]
    EventHub(#[from] wlf_core::event_hub::Error),
}

impl ComponentApi for BinlogFilter {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Transformer
    }
}

impl BinlogFilter {
    pub fn new(id: impl Into<String>, destination: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            destination: destination.into(),
        }
    }
    pub async fn start_filtering(self, hub: Arc<EventHub>) -> Result<(), Error> {
        while let Ok(event) = hub.poll_event(self.id()).await {
            info!("new event:\n{event:#?}");

            let Some(Value::String(table)) = event.value.pointer("/sql/table") else {
                continue;
            };

            println!("{table}");
            // filtering ..

            hub.send_event(event, self.destination.as_str()).await?;
        }
        Ok(())
    }
}
