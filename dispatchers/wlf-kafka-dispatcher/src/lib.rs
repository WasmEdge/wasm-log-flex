use std::{collections::BTreeMap, sync::Arc};

use chrono::Utc;
use regex::{Captures, Regex};
use rskafka::{
    client::{
        partition::{Compression, UnknownTopicHandling},
        ClientBuilder,
    },
    record::Record,
};
use serde_json::Value;
use thiserror::Error;
use tracing::{error, info};
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

    pub async fn start_dispatching(self, router: Arc<EventRouter>) -> Result<(), Error> {
        let client = ClientBuilder::new(self.bootstrap_brokers.clone())
            .build()
            .await?;
        let controller_client = client.controller_client()?;
        let mut topics_cache = client.list_topics().await?;
        let replacer = Regex::new(r"%\{(?P<path>.+?)\}").expect("can't create topic replacer");
        while let Ok(event) = router.poll_event(self.id()).await {
            info!("receive new event:\n{event:#?}");
            // get the topic
            let topic_name = {
                let mut succeeded = true;
                let topic_name = replacer
                    .replace_all(&self.topic, |caps: &Captures| {
                        let path = &caps["path"];
                        if let Some(Value::String(value)) = event.value.pointer(path) {
                            value.to_string()
                        } else {
                            error!("no {path} field or {path} is not string, event: {event:?}");
                            succeeded = false;
                            String::new()
                        }
                    })
                    .to_string();
                if !succeeded {
                    continue;
                }
                topic_name
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
