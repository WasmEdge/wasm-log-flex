use std::{collections::BTreeMap, sync::Arc};

use chrono::Utc;
use rskafka::{
    client::{
        partition::{Compression, UnknownTopicHandling},
        ClientBuilder,
    },
    record::Record,
};
use thiserror::Error;
use tracing::{error, info};
use wlf_core::{
    event_hub::{EventHub, EventHubApi},
    ComponentApi, ComponentKind, Value,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("event hub error, {0}")]
    EventHub(#[from] wlf_core::event_hub::Error),
    #[error("kafka client error, {0}")]
    KafkaClient(#[from] rskafka::client::error::Error),
    #[error("serialize/deserialize error, {0}")]
    Serde(#[from] serde_json::Error),
}

pub struct KafkaDispatcher {
    id: String,
    topic: String,
    bootstrap_brokers: Vec<String>,
}

impl ComponentApi for KafkaDispatcher {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Dispatcher
    }
}

impl KafkaDispatcher {
    pub fn new(id: impl Into<String>, bootstrap_brokers: Vec<String>) -> Self {
        Self {
            id: id.into(),
            topic: default_topic().to_string(),
            bootstrap_brokers,
        }
    }

    pub fn set_topic(&mut self, topic: impl Into<String>) {
        self.topic = topic.into();
    }

    pub async fn start_dispatching(self, hub: Arc<EventHub>) -> Result<(), Error> {
        let client = ClientBuilder::new(self.bootstrap_brokers.clone())
            .build()
            .await?;
        let controller_client = client.controller_client()?;
        let mut topics_cache = client.list_topics().await?;
        while let Ok(event) = hub.poll_event(self.id()).await {
            info!("receive new event:\n{event:#?}");
            // get the topic
            let topic_name = if self.topic == default_topic() {
                default_topic().to_owned()
            } else {
                let Some(Value::String(database)) = event.value.pointer("/meta/database") else {
                    error!("no `database` field or `database` is not string, event: {event:?}");
                    continue;
                };
                let Some(Value::String(table)) = event.value.pointer("/sql/table") else {
                    error!("no `table` field or `table` is not string, event: {event:?}");
                    continue;
                };
                self.topic
                    .to_owned()
                    .replace("%{database}", database)
                    .replace("%{table}", table)
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

const fn default_topic() -> &'static str {
    "wasm-log-flex"
}
