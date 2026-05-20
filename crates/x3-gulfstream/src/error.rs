//! Gulfstream Error Types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GulfstreamError {
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    #[error("Mempool error: {0}")]
    MempoolError(String),
    
    #[error("Forwarder error: {0}")]
    ForwarderError(String),
    
    #[error("Leader schedule error: {0}")]
    LeaderError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Not started: {0}")]
    NotStarted(String),
}

pub type GulfstreamResult<T> = Result<T, GulfstreamError>;