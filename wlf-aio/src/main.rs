use std::{error::Error, fs::File, io::BufReader, path::PathBuf, sync::Arc};

use clap::{command, Parser};
use tracing::error;
use wlf_core::event_router::{EventRouter, EventRouterApi};

use crate::config::Config;

mod config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config: Config = serde_yaml::from_reader(BufReader::new(File::open(cli.config)?))?;

    let mut router = EventRouter::new();
    for c in &config.collectors {
        router.register_component(c.as_component());
    }
    for t in &config.transformers {
        router.register_component(t.as_component());
    }
    for d in &config.dispatchers {
        router.register_component(d.as_component());
    }
    let router = Arc::new(router);

    let mut handles = vec![];
    for c in config.collectors {
        let router_c = Arc::clone(&router);
        let handle = tokio::spawn(async move {
            if let Err(e) = c.as_component().run(router_c).await {
                error!(e);
            }
        });
        handles.push(handle);
    }
    for t in config.transformers {
        let router_c = Arc::clone(&router);
        let handle = tokio::spawn(async move {
            if let Err(e) = t.as_component().run(router_c).await {
                error!(e);
            }
        });
        handles.push(handle);
    }
    for d in config.dispatchers {
        let router_c = Arc::clone(&router);
        let handle = tokio::spawn(async move {
            if let Err(e) = d.as_component().run(router_c).await {
                error!(e);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("something went wrong");
    }

    Ok(())
}
