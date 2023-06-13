#[derive(Debug)]
pub struct BinlogEvent {
    pub sql: String,
}

pub struct BinlogParsedEvent {
    pub binlog_event: BinlogEvent,
    pub table: String,
}
