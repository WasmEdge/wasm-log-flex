use std::collections::HashMap;

use async_trait::async_trait;
use flume::{RecvError, SendError};

use crate::{event::Event, ComponentApi, ComponentKind};
use thiserror::Error;
use tracing::error;

#[async_trait]
pub trait EventRouterApi {
    async fn send_event(&self, event: Event, component_id: &str) -> Result<(), Error>;
    async fn poll_event(&self, component_id: &str) -> Result<Event, Error>;
    fn register_component(&mut self, collector: &dyn ComponentApi);
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("no such component {0}")]
    NoSuchComponent(String),
    #[error("wrong component kind")]
    WrongComponentKind,
    #[error("internal error, {0}")]
    Internal(String),
}

pub struct EventRouter {
    registry: HashMap<String, ComponentRecord>,
}

pub enum ComponentRecord {
    Collector,
    Transformer {
        tx: flume::Sender<Event>,
        rx: flume::Receiver<Event>,
    },
    Dispatcher {
        tx: flume::Sender<Event>,
        rx: flume::Receiver<Event>,
    },
}

impl EventRouter {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }
}

#[async_trait]
impl EventRouterApi for EventRouter {
    async fn send_event(&self, event: Event, component_id: &str) -> Result<(), Error> {
        let r = self.registry.get(component_id).ok_or_else(|| {
            error!("can't send event to component {component_id}, component does not exist");
            Error::NoSuchComponent(component_id.to_string())
        })?;
        let tx = match r {
            ComponentRecord::Collector => {
                error!("can't send an event to collector");
                return Err(Error::WrongComponentKind);
            }
            ComponentRecord::Dispatcher { tx, .. } | ComponentRecord::Transformer { tx, .. } => tx,
        };
        tx.send_async(event).await?;
        Ok(())
    }
    async fn poll_event(&self, component_id: &str) -> Result<Event, Error> {
        let r = self.registry.get(component_id).ok_or_else(|| {
            error!("can't send event to component {component_id}, component does not exist");
            Error::NoSuchComponent(component_id.to_string())
        })?;
        let rx = match r {
            ComponentRecord::Collector => {
                error!("collector can't receive an event");
                return Err(Error::WrongComponentKind);
            }
            ComponentRecord::Dispatcher { rx, .. } | ComponentRecord::Transformer { rx, .. } => rx,
        };
        Ok(rx.recv_async().await?)
    }
    fn register_component(&mut self, component: &dyn ComponentApi) {
        if self.registry.contains_key(component.id()) {
            error!("component {} has already been registered", component.id());
            return;
        }
        match component.kind() {
            ComponentKind::Collector => {
                self.registry
                    .insert(component.id().to_owned(), ComponentRecord::Collector);
            }
            ComponentKind::Transformer => {
                let (tx, rx) = flume::unbounded();
                self.registry.insert(
                    component.id().to_owned(),
                    ComponentRecord::Transformer { tx, rx },
                );
            }
            ComponentKind::Dispatcher => {
                let (tx, rx) = flume::unbounded();
                self.registry.insert(
                    component.id().to_owned(),
                    ComponentRecord::Dispatcher { tx, rx },
                );
            }
        }
    }
}

impl Default for EventRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_value: SendError<T>) -> Self {
        Error::Internal("failed to send event".to_string())
    }
}

impl From<RecvError> for Error {
    fn from(_value: RecvError) -> Self {
        Error::Internal("failed to receive event".to_string())
    }
}
