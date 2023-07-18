use std::sync::Arc;

use redis::{
    AsyncCommands, ConnectionAddr, ConnectionInfo, IntoConnectionInfo, RedisConnectionInfo,
    RedisError,
};
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
    mode: Mode,
    config: Config,
}

pub struct Config {
    host: String,
    port: u16,
    auth: Option<String>,
    database_number: u8,
}

impl IntoConnectionInfo for Config {
    fn into_connection_info(self) -> redis::RedisResult<ConnectionInfo> {
        Ok(ConnectionInfo {
            addr: ConnectionAddr::Tcp(self.host, self.port),
            redis: RedisConnectionInfo {
                db: self.database_number as i64,
                username: None,
                password: self.auth,
            },
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 6379,
            auth: None,
            database_number: 0,
        }
    }
}

pub enum Mode {
    LPush { key: String },
    RPush { key: String },
    Pub { channel: String },
    XADD { key: String },
}

impl Default for Mode {
    fn default() -> Self {
        Self::RPush {
            key: "wlf".to_string(),
        }
    }
}

impl RedisDispatcher {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            config: Default::default(),
            mode: Default::default(),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub fn set_password(&mut self, password: impl Into<String>) -> &mut Self {
        self.config.auth = Some(password.into());
        self
    }

    pub async fn start_dispatching(self, router: Arc<EventRouter>) -> Result<(), Error> {
        let mut redis_client = redis::Client::open(self.config)?
            .get_async_connection()
            .await?;

        while let Ok(event) = router.poll_event(&self.id).await {
            info!("receive new event:\n{event:#?}");

            match &self.mode {
                Mode::LPush { key } => {
                    let key = substitute_with_event(key, &event).map_err(Error::TopicName)?;
                    let value = serde_json::to_string(&event)?;
                    redis_client.lpush(key, value).await?;
                }
                Mode::RPush { key } => {
                    let key = substitute_with_event(key, &event).map_err(Error::TopicName)?;
                    let value = serde_json::to_string(&event)?;
                    redis_client.rpush(key, value).await?;
                }
                Mode::Pub { channel } => {
                    let channel =
                        substitute_with_event(channel, &event).map_err(Error::TopicName)?;
                    let value = serde_json::to_string(&event)?;
                    redis_client.publish(channel, value).await?;
                }
                Mode::XADD { key } => {
                    let key = substitute_with_event(key, &event).map_err(Error::TopicName)?;
                    let value = serde_json::to_string(&event)?;
                    redis_client
                        .xadd(key, "*", &[("event".to_string(), value)])
                        .await?;
                }
            }
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

#[cfg(test)]
mod tests {
    use std::{error::Error, time::Duration};

    use serde_json::json;
    use wlf_core::{Event, EventMeta};

    use super::*;

    #[tokio::test]
    async fn run() -> Result<(), Box<dyn Error>> {
        let _ = tracing_subscriber::fmt::try_init();

        let mut dispatcher = RedisDispatcher::new("redis_dispatcher");

        // dispatcher.set_mode(Mode::RPush {
        //     key: r"wlf_%{/file}".to_string(),
        // });
        // dispatcher.set_mode(Mode::XADD {
        //     key: r"wlf_%{/file}".to_string(),
        // });
        dispatcher.set_mode(Mode::Pub {
            channel: r"wlf_%{/file}".to_string(),
        });

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
