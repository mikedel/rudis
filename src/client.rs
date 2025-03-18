use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::{BytesMut, Bytes};
use std::time::Duration;
use thiserror::Error;

use crate::protocol::{RedisValue, serialize_response};

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("connection error: {0}")]
    ConnectionError(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    ProtocolError(String),
    #[error("timeout")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, ClientError>;

pub struct Client {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Client {
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            stream,
            buffer: BytesMut::with_capacity(4096),
        })
    }
    
    pub async fn get(&mut self, key: &str) -> Result<Option<Bytes>> {
        let cmd = format!("GET {}\r\n", key);
        self.stream.write_all(cmd.as_bytes()).await?;
        
        let mut response_buf = [0u8; 1024];
        let n = self.stream.read(&mut response_buf).await?;
        
        if n == 0 {
            return Err(ClientError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Connection closed",
            )));
        }
        
        let response = std::str::from_utf8(&response_buf[..n])
            .map_err(|_| ClientError::ProtocolError("Invalid UTF-8".to_string()))?;
        
        // Handle nil response
        if response.starts_with("$-1") {
            return Ok(None);
        }
        
        // Simple parsing
        if response.starts_with("$") {
            // Extract the byte string
            let parts: Vec<&str> = response.split("\r\n").collect();
            if parts.len() >= 3 {
                return Ok(Some(Bytes::from(parts[1].as_bytes().to_vec())));
            }
        }
        
        Err(ClientError::ProtocolError(format!("Unexpected response: {}", response)))
    }
    
    pub async fn set(&mut self, key: &str, value: &[u8], ttl: Option<u64>) -> Result<()> {
        let mut cmd = format!("SET {} {}", key, std::str::from_utf8(value)
            .map_err(|_| ClientError::ProtocolError("Invalid UTF-8 in value".to_string()))?);
        
        if let Some(ttl) = ttl {
            cmd = format!("{} EX {}", cmd, ttl);
        }
        
        cmd = format!("{}\r\n", cmd);
        self.stream.write_all(cmd.as_bytes()).await?;
        
        let mut response_buf = [0u8; 1024];
        let n = self.stream.read(&mut response_buf).await?;
        
        if n == 0 {
            return Err(ClientError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Connection closed",
            )));
        }
        
        let response = std::str::from_utf8(&response_buf[..n])
            .map_err(|_| ClientError::ProtocolError("Invalid UTF-8".to_string()))?;
            
        if response.contains("OK") {
            Ok(())
        } else {
            Err(ClientError::ProtocolError(format!("Unexpected response: {}", response)))
        }
    }
    
    pub async fn pop(&mut self) -> Result<Option<(String, Bytes)>> {
        let cmd = "POP\r\n";
        self.stream.write_all(cmd.as_bytes()).await?;
        
        let mut response_buf = [0u8; 1024];
        let n = self.stream.read(&mut response_buf).await?;
        
        if n == 0 {
            return Err(ClientError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Connection closed",
            )));
        }
        
        let response = std::str::from_utf8(&response_buf[..n])
            .map_err(|_| ClientError::ProtocolError("Invalid UTF-8".to_string()))?;
        
        if response.starts_with("$-1") || response.starts_with("*-1") {
            return Ok(None);
        }
        
        // TODO: Implement proper RESP parsing
        if response.starts_with("*") {
            let parts: Vec<&str> = response.split("\r\n").collect();
            if parts.len() >= 5 {
                let key = parts[2].to_string();
                let value = Bytes::from(parts[4].as_bytes().to_vec());
                return Ok(Some((key, value)));
            }
        }
        
        Err(ClientError::ProtocolError(format!("Unexpected response: {}", response)))
    }
}