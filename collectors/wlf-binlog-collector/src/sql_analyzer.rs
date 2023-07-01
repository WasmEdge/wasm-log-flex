use serde_json::{Map, Value};
use sqlparser::{
    ast::Statement,
    dialect::MySqlDialect,
    parser::{Parser, ParserError},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to parse sql, {0}")]
    Parse(#[from] ParserError),
    #[error("{0}")]
    Other(String),
}

pub(super) struct SqlAnalyzer;

impl SqlAnalyzer {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn analyze(&self, sql: &str) -> Result<Value, Error> {
        // parse
        let mut ast = Parser::parse_sql(&MySqlDialect {}, sql)?;
        if ast.len() != 1 {
            return Err(Error::Other("multiple statements in one sql".to_string()));
        }
        let st = ast.remove(0);

        // extract info
        let mut properties: Map<String, Value> = Map::new();
        properties.insert("statement".to_string(), sql.into());
        match st {
            Statement::Insert { table_name, .. } => {
                properties.insert("type".to_string(), "insert".into());
                properties.insert("table".to_string(), table_name.to_string().into());
            }
            Statement::CreateTable { name, .. } => {
                properties.insert("type".to_string(), "table-create".into());
                properties.insert("table".to_string(), name.to_string().into());
            }
            _ => {}
        }

        Ok(properties.into())
    }
}
