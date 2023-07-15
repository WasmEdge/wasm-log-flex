use std::sync::Arc;

use redis::{AsyncCommands, RedisError};
use thiserror::Error;
use tracing::info;
use utils::substitute_with_event;
use wlf_core::{
    event_router::{EventRouter, EventRouterApi},
    ComponentApi, ComponentKind,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("redis error, {0}")]
    Redis(#[from] RedisError),
    #[error("failed to generate redis key, {0}")]
    TopicName(String),
    #[error("serialize/deserialize error, {0}")]
    Serde(#[from] serde_json::Error),
}

pub struct RedisDispatcher {
    id: String,
    key: String,
    url: String,
}

impl RedisDispatcher {
    pub fn new(id: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            key: default_redis_key().into(),
            url: url.into(),
        }
    }

    pub fn set_key_template(&mut self, key: impl Into<String>) {
        self.key = key.into();
    }

    pub async fn start_dispatching(self, router: Arc<EventRouter>) -> Result<(), Error> {
        let mut redis_client = redis::Client::open(self.url)?
            .get_async_connection()
            .await?;

        while let Ok(event) = router.poll_event(&self.id).await {
            info!("receive new event:\n{event:#?}");

            let key = substitute_with_event(&self.key, &event).map_err(Error::TopicName)?;
            let value = serde_json::to_string(&event)?;

            redis_client.rpush(key, value).await?;
        }

        Ok(())
    }
}

impl ComponentApi for RedisDispatcher {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn kind(&self) -> ComponentKind {
        ComponentKind::Dispatcher
    }
}

pub const fn default_redis_key() -> &'static str {
    "wlf"
}

#[cfg(test)]
mod tests {
    use std::{error::Error, time::Duration};

    use serde_json::json;
    use wlf_core::{Event, EventMeta};

    use super::*;

    #[tokio::test]
    async fn run() -> Result<(), Box<dyn Error>> {
        let _ = tracing_subscriber::fmt::try_init();

        let mut dispatcher = RedisDispatcher::new("redis_dispatcher", "redis://127.0.0.1/");
        dispatcher.set_key_template(r"wlf_%{/file}");

        let mut router = EventRouter::new();
        router.register_component(&dispatcher);
        let router = Arc::new(router);

        let router_c = Arc::clone(&router);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;
                router_c
                    .send_event(
                        Event {
                            value: json!({
                                "file": "log1",
                                "msg": "hello"
                            }),
                            meta: EventMeta {},
                        },
                        "redis_dispatcher",
                    )
                    .await
                    .unwrap();
                router_c
                    .send_event(
                        Event {
                            value: json!({
                                "file": "log2",
                                "msg": "hello"
                            }),
                            meta: EventMeta {},
                        },
                        "redis_dispatcher",
                    )
                    .await
                    .unwrap();
            }
        });

        dispatcher.start_dispatching(router).await?;

        Ok(())
    }
}
