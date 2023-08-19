use std::sync::Arc;

use async_trait::async_trait;
use elasticsearch::{http::transport::Transport, Elasticsearch, IndexParts};
use serde::Deserialize;
use thiserror::Error;
use tracing::{error, info};
use utils::substitute_with_event;
use wlf_core::{
    event_router::{EventRouter, EventRouterApi},
    ComponentApi, ComponentKind,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("serialize/deserialize error, {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Deserialize, Debug)]
pub struct ElasticsearchDispatcher {
    pub id: String,
    #[serde(default = "default_url")]
    pub url: String,
    #[serde(default = "default_index")]
    pub index: String,
}

pub fn default_url() -> String {
    "http://localhost:9200".to_string()
}

pub fn default_index() -> String {
    "wlf".to_string()
}

#[async_trait]
impl ComponentApi for ElasticsearchDispatcher {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Dispatcher
    }

    async fn run(&self, router: Arc<EventRouter>) -> Result<(), Box<dyn std::error::Error>> {
        let transport = Transport::single_node(&self.url)?;
        let client = Elasticsearch::new(transport);

        while let Ok(event) = router.poll_event(self.id()).await {
            info!("{} receives new event:\n\t{event:?}", self.id);

            let Ok(index) = substitute_with_event(&self.index, &event) else {
                continue;
            };

            let resp = client
                .index(IndexParts::Index(&index))
                .body(event)
                .send()
                .await?;

            info!("event is dispatched to elasticsearch, resp: {resp:?}");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use utils::test_utils::DummyComponent;

    use super::*;

    #[tokio::test]
    async fn collect() {
        let collector = ElasticsearchDispatcher {
            id: "es".to_string(),
            url: default_url(),
            index: default_index(),
        };

        let dummy_dispatcher = DummyComponent::new("dispatcher", ComponentKind::Dispatcher);

        let mut router = EventRouter::new();
        router.register_component(&collector);
        router.register_component(&dummy_dispatcher);
        let router = Arc::new(router);

        let router_c = Arc::clone(&router);
        tokio::spawn(async move {
            collector
                .run(Arc::clone(&router_c))
                .await
                .expect("failed to run dispatcher");
        });

        while let Ok(event) = router.poll_event("dispatcher").await {
            println!("{event:#?}");
        }
    }
}
