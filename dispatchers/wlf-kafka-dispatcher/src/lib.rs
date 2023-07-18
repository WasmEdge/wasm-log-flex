use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use rskafka::{
    client::{
        partition::{Compression, UnknownTopicHandling},
        ClientBuilder,
    },
    record::Record,
};
use serde::Deserialize;
use thiserror::Error;
use tracing::{error, info, warn};
use utils::substitute_with_event;
use wlf_core::{
    event_router::{EventRouter, EventRouterApi},
    ComponentApi, ComponentKind,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("event router error, {0}")]
    EventRouter(#[from] wlf_core::event_router::Error),
    #[error("kafka client error, {0}")]
    KafkaClient(#[from] rskafka::client::error::Error),
    #[error("serialize/deserialize error, {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Deserialize, Debug)]
pub struct KafkaDispatcher {
    id: String,
    #[serde(default = "default_topic")]
    topic: String,
    bootstrap_brokers: Vec<String>,
}

#[async_trait]
impl ComponentApi for KafkaDispatcher {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Dispatcher
    }

    async fn run(&self, router: Arc<EventRouter>) -> Result<(), Box<dyn std::error::Error>> {
        let client = ClientBuilder::new(self.bootstrap_brokers.clone())
            .build()
            .await?;
        let controller_client = client.controller_client()?;
        let mut topics_cache = client.list_topics().await?;
        while let Ok(event) = router.poll_event(self.id()).await {
            info!("receive new event:\n{event:#?}");
            // get the topic
            let Ok(topic_name) = substitute_with_event(&self.topic, &event) else {
                warn!("can't generate topic_name for event");
                continue;
            };

            // create the topic in kafka if topic does not exist
            if !topics_cache.iter().any(|topic| topic.name == topic_name) {
                controller_client
                    .create_topic(topic_name.clone(), 1, 1, 5_000)
                    .await?;
                topics_cache = client.list_topics().await?;
            }

            // get the partition client
            let partition_client = client
                .partition_client(topic_name.clone(), 0, UnknownTopicHandling::Retry)
                .await?;

            // create record
            let record = Record {
                key: None,
                value: Some(serde_json::to_vec(&event)?),
                headers: BTreeMap::new(),
                timestamp: Utc::now(),
            };
            partition_client
                .produce(vec![record], Compression::default())
                .await?;

            // dispatch event to corresponding kafka table
            info!("event is dispatched to topic {}", topic_name);
        }
        Ok(())
    }
}

fn default_topic() -> String {
    "wasm-log-flex".to_string()
}
