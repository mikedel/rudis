use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use bytes::{BytesMut, Bytes};
use std::sync::Arc;
use std::time::Duration;
use log::{info, error, debug};

use crate::storage::{Storage, StorageError};
use crate::protocol::{parse_command, serialize_response, RedisCommand, RedisValue};

pub struct Server {
    storage: Arc<Storage>,
    addr: String,
}

impl Server {
    pub fn new(addr: String) -> Self {
        Self {
            storage: Arc::new(Storage::new()),
            addr,
        }
    }
    
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("Rudis server listening on {}", self.addr);
        
        // Start background task for expired key cleanup
        let storage_clone = Arc::clone(&self.storage);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let removed = storage_clone.cleanup_expired();
                if removed > 0 {
                    debug!("Removed {} expired keys", removed);
                }
            }
        });
        
        loop {
            let (socket, addr) = listener.accept().await?;
            info!("Client connected: {}", addr);
            
            let storage = Arc::clone(&self.storage);
            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, storage).await {
                    error!("Error handling client {}: {}", addr, e);
                }
            });
        }
    }
}

async fn handle_client(
    mut socket: TcpStream, 
    storage: Arc<Storage>
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, writer) = socket.split();
    let mut reader = tokio::io::BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut buffer = BytesMut::with_capacity(4096);
    
    loop {
        let bytes_read = reader.read_buf(&mut buffer).await?;
        if bytes_read == 0 {
            // Client disconnected
            break;
        }
        
        match parse_command(&mut buffer) {
            Ok(Some(cmd)) => {
                let response = execute_command(cmd, &storage).await;
                let serialized = serialize_response(response);
                writer.write_all(&serialized).await?;
                writer.flush().await?;
                
                // Clear the buffer for the next command
                buffer.clear();
            },
            Ok(None) => {
                // Incomplete command, continue reading
                continue;
            },
            Err(e) => {
                let error_response = serialize_response(RedisValue::Error(format!("Error: {}", e)));
                writer.write_all(&error_response).await?;
                writer.flush().await?;
                buffer.clear();
            }
        }
    }
    
    Ok(())
}

async fn execute_command(cmd: RedisCommand, storage: &Storage) -> RedisValue {
    match cmd {
        RedisCommand::Get { key } => {
            match storage.get(&key) {
                Ok(value) => RedisValue::Bytes(value),
                Err(StorageError::KeyNotFound) => RedisValue::Nil,
                Err(StorageError::KeyExpired) => RedisValue::Nil,
                Err(e) => RedisValue::Error(format!("Get error: {}", e)),
            }
        },
        RedisCommand::Set { key, value, ttl } => {
            let ttl = ttl.map(Duration::from_secs);
            match storage.set(key, value, ttl) {
                Ok(_) => RedisValue::String("OK".to_string()),
                Err(e) => RedisValue::Error(format!("Set error: {}", e)),
            }
        },
        RedisCommand::Delete { key } => {
            match storage.delete(&key) {
                Ok(_) => RedisValue::Integer(1),
                Err(StorageError::KeyNotFound) => RedisValue::Integer(0),
                Err(e) => RedisValue::Error(format!("Delete error: {}", e)),
            }
        },
        RedisCommand::Keys { pattern } => {
            let keys = storage.keys(&pattern);
            let values = keys.into_iter()
                .map(|k| RedisValue::String(k))
                .collect();
            RedisValue::Array(values)
        },
        RedisCommand::Pop => {
            match storage.pop_fifo() {
                Ok((key, value)) => RedisValue::Array(vec![
                    RedisValue::String(key),
                    RedisValue::Bytes(value),
                ]),
                Err(StorageError::KeyNotFound) => RedisValue::Nil,
                Err(e) => RedisValue::Error(format!("Pop error: {}", e)),
            }
        },
        RedisCommand::Ping => RedisValue::String("PONG".to_string()),
        RedisCommand::Info => {
            let info = "# Rudis\r\nversion:0.1.0\r\nrust_version:1.68.0\r\n";
            RedisValue::String(info.to_string())
        },
    }
}