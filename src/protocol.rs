use bytes::{Bytes, BytesMut, Buf, BufMut};
use std::io;
use thiserror::Error;

#[derive(Debug)]
pub enum RedisCommand {
    Get { key: String },
    Set { key: String, value: Bytes, ttl: Option<u64> },
    Delete { key: String },
    Pop,
    Ping,
    Info,
    Keys { pattern: String },
}

#[derive(Debug)]
pub enum RedisValue {
    String(String),
    Bytes(Bytes),
    Integer(i64),
    Nil,
    Error(String),
    Array(Vec<RedisValue>),
}

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("invalid protocol format")]
    InvalidFormat,
    #[error("invalid command")]
    InvalidCommand,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

type Result<T> = std::result::Result<T, ProtocolError>;

pub fn parse_command(buffer: &mut BytesMut) -> Result<Option<RedisCommand>> {
    if buffer.is_empty() {
        return Ok(None);
    }
    
    // Debug the raw buffer
    println!("Raw buffer: {:?}", buffer);
    
    // Check if we have a complete command (ending with \r\n)
    if !buffer.windows(2).any(|window| window == b"\r\n") {
        return Ok(None);
    }
    
    // Convert to string for easier debugging
    let cmd_str = std::str::from_utf8(buffer).map_err(|_| ProtocolError::InvalidFormat)?;
    println!("Received command: {:?}", cmd_str);
    
    // Handle RESP protocol
    if cmd_str.starts_with('*') {
        // This is RESP array format
        let lines: Vec<&str> = cmd_str.split("\r\n").collect();
        println!("Split lines: {:?}", lines);
        
        if lines.len() < 3 {
            return Err(ProtocolError::InvalidFormat);
        }
        
        // Extract command parts
        let mut parts = Vec::new();
        let mut i = 1; // Skip the first line (*n)
        
        while i < lines.len() {
            if lines[i].starts_with('$') && i + 1 < lines.len() {
                parts.push(lines[i + 1]);
                i += 2;
            } else {
                i += 1;
            }
        }
        
        println!("Parsed RESP parts: {:?}", parts);
        
        if parts.is_empty() {
            return Err(ProtocolError::InvalidFormat);
        }
        
        // Parse command
        match parts[0].to_uppercase().as_str() {
            "KEYS" => {
                if parts.len() < 2 {
                    return Err(ProtocolError::InvalidFormat);
                }
                println!("Processing KEYS command with pattern: {}", parts[1]);
                Ok(Some(RedisCommand::Keys {
                    pattern: parts[1].to_string()
                }))
            },
            "GET" => {
                if parts.len() < 2 {
                    return Err(ProtocolError::InvalidFormat);
                }
                Ok(Some(RedisCommand::Get { 
                    key: parts[1].to_string() 
                }))
            },
            "SET" => {
                if parts.len() < 3 {
                    return Err(ProtocolError::InvalidFormat);
                }
                
                let mut ttl = None;
                if parts.len() > 4 && parts[3].to_uppercase() == "EX" {
                    ttl = parts[4].parse::<u64>().ok();
                }
                
                Ok(Some(RedisCommand::Set { 
                    key: parts[1].to_string(),
                    value: Bytes::from(parts[2].as_bytes().to_vec()),
                    ttl,
                }))
            },
            "DEL" => {
                if parts.len() < 2 {
                    return Err(ProtocolError::InvalidFormat);
                }
                Ok(Some(RedisCommand::Delete { 
                    key: parts[1].to_string() 
                }))
            },
            "POP" => Ok(Some(RedisCommand::Pop)),
            "PING" => Ok(Some(RedisCommand::Ping)),
            "INFO" => Ok(Some(RedisCommand::Info)),
            _ => {
                println!("Unknown command: {}", parts[0]);
                Err(ProtocolError::InvalidCommand)
            },
        }
    } else {
        // Simple text protocol
        let parts: Vec<&str> = cmd_str.trim().split_whitespace().collect();
        println!("Parsed simple parts: {:?}", parts);
        
        if parts.is_empty() {
            return Err(ProtocolError::InvalidFormat);
        }
        
        // Parse command
        match parts[0].to_uppercase().as_str() {
            "KEYS" => {
                if parts.len() < 2 {
                    return Err(ProtocolError::InvalidFormat);
                }
                println!("Processing KEYS command with pattern: {}", parts[1]);
                Ok(Some(RedisCommand::Keys {
                    pattern: parts[1].to_string()
                }))
            },
            "GET" => {
                if parts.len() < 2 {
                    return Err(ProtocolError::InvalidFormat);
                }
                Ok(Some(RedisCommand::Get { 
                    key: parts[1].to_string() 
                }))
            },
            "SET" => {
                if parts.len() < 3 {
                    return Err(ProtocolError::InvalidFormat);
                }
                
                let mut ttl = None;
                if parts.len() > 4 && parts[3].to_uppercase() == "EX" {
                    ttl = parts[4].parse::<u64>().ok();
                }
                
                Ok(Some(RedisCommand::Set { 
                    key: parts[1].to_string(),
                    value: Bytes::from(parts[2].as_bytes().to_vec()),
                    ttl,
                }))
            },
            "DEL" => {
                if parts.len() < 2 {
                    return Err(ProtocolError::InvalidFormat);
                }
                Ok(Some(RedisCommand::Delete { 
                    key: parts[1].to_string() 
                }))
            },
            "POP" => Ok(Some(RedisCommand::Pop)),
            "PING" => Ok(Some(RedisCommand::Ping)),
            "INFO" => Ok(Some(RedisCommand::Info)),
            _ => {
                println!("Unknown command: {}", parts[0]);
                Err(ProtocolError::InvalidCommand)
            },
        }
    }
}

pub fn serialize_response(value: RedisValue) -> Bytes {
    let mut buf = BytesMut::new();
    
    match value {
        RedisValue::String(s) => {
            buf.put_u8(b'+');
            buf.put_slice(s.as_bytes());
            buf.put_slice(b"\r\n");
        },
        RedisValue::Integer(i) => {
            buf.put_u8(b':');
            buf.put_slice(i.to_string().as_bytes());
            buf.put_slice(b"\r\n");
        },
        RedisValue::Bytes(b) => {
            buf.put_u8(b'$');
            buf.put_slice(b.len().to_string().as_bytes());
            buf.put_slice(b"\r\n");
            buf.put_slice(&b);
            buf.put_slice(b"\r\n");
        },
        RedisValue::Nil => {
            buf.put_slice(b"$-1\r\n");
        },
        RedisValue::Error(e) => {
            buf.put_u8(b'-');
            buf.put_slice(e.as_bytes());
            buf.put_slice(b"\r\n");
        },
        RedisValue::Array(arr) => {
            buf.put_u8(b'*');
            buf.put_slice(arr.len().to_string().as_bytes());
            buf.put_slice(b"\r\n");
            for item in arr {
                let serialized = serialize_response(item);
                buf.put_slice(&serialized);
            }
        }
    }
    
    buf.freeze()
}