use std::collections::HashMap;

use async_trait::async_trait;

use crate::{ComponentApi, ComponentKind, Event};

#[async_trait]
pub trait EventHubApi {
    async fn send_event(&self, event: Event, component_id: &str) -> Result<(), Error>;
    async fn poll_event(&self, component_id: &str) -> Result<Event, Error>;
    fn register_component(&mut self, collector: &impl ComponentApi);
}

#[derive(Debug)]
pub enum Error {
    NoSuchComponent,
    WrongComponentKind,
    Internal,
}

pub struct EventHub {
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

impl EventHub {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }
}

#[async_trait]
impl EventHubApi for EventHub {
    async fn send_event(&self, event: Event, component_id: &str) -> Result<(), Error> {
        let r = self.registry.get(component_id).ok_or_else(|| {
            eprintln!("can't send event to component {component_id}, component does not exist or is a Collector");
            Error::NoSuchComponent
        })?;
        let tx = match r {
            ComponentRecord::Collector => {
                eprint!("can't send an event to collector");
                return Err(Error::WrongComponentKind);
            }
            ComponentRecord::Dispatcher { tx, .. } | ComponentRecord::Transformer { tx, .. } => tx,
        };
        tx.send_async(event).await.map_err(|e| {
            eprintln!("can't send event to next component, internal channel error, {e}");
            Error::Internal
        })
    }
    async fn poll_event(&self, component_id: &str) -> Result<Event, Error> {
        let r = self.registry.get(component_id).ok_or_else(|| {
            eprintln!("can't send event to component {component_id}, component does not exist or is a Collector");
            Error::NoSuchComponent
        })?;
        let rx = match r {
            ComponentRecord::Collector => {
                eprint!("collector can't receive an event");
                return Err(Error::WrongComponentKind);
            }
            ComponentRecord::Dispatcher { rx, .. } | ComponentRecord::Transformer { rx, .. } => rx,
        };
        rx.recv_async().await.map_err(|e| {
            eprintln!(
                "can't receive event for component {component_id}, internal channel error, {e}"
            );
            Error::Internal
        })
    }
    fn register_component(&mut self, component: &impl ComponentApi) {
        if self.registry.contains_key(component.id()) {
            eprintln!("component has already been registered");
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

impl Default for EventHub {
    fn default() -> Self {
        Self::new()
    }
}
