use thiserror::Error;

use crate::sql_analyzer;

#[derive(Error, Debug)]
pub enum Error {
    #[error("binlog client error, {0}")]
    BinlogClient(#[from] mysql_cdc::errors::Error),
    #[error("event router error, {0}")]
    EventRouter(#[from] wlf_core::event_router::Error),
    #[error("failed to analyze sql statement, {0}")]
    SqlAnalyzer(#[from] sql_analyzer::Error),
    #[error("{0}")]
    Other(String),
}
