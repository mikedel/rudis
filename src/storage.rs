use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use bytes::Bytes;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("key not found")]
    KeyNotFound,
    #[error("key expired")]
    KeyExpired,
    #[error("value deserialization failed")]
    DeserializationError,
}

pub type Result<T> = std::result::Result<T, StorageError>;

struct ValueEntry {
    data: Bytes,
    expiry: Option<Instant>,
    insertion_time: Instant,
}

pub struct Storage {
    map: Arc<DashMap<String, ValueEntry>>,
    fifo_keys: Arc<DashMap<Instant, String>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            map: Arc::new(DashMap::new()),
            fifo_keys: Arc::new(DashMap::new()),
        }
    }

    pub fn set(&self, key: String, value: Bytes, ttl: Option<Duration>) -> Result<()> {
        let now = Instant::now();
        let expiry = ttl.map(|duration| now + duration);
        
        let entry = ValueEntry {
            data: value,
            expiry,
            insertion_time: now,
        };
        
        // Store the value
        self.map.insert(key.clone(), entry);
        
        // Add to FIFO queue
        self.fifo_keys.insert(now, key);
        
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Bytes> {
        let entry = self.map.get(key).ok_or(StorageError::KeyNotFound)?;
        
        // Check if key has expired
        if let Some(expiry) = entry.expiry {
            if Instant::now() > expiry {
                // Remove expired key
                self.map.remove(key);
                return Err(StorageError::KeyExpired);
            }
        }
        
        Ok(entry.data.clone())
    }

    pub fn pop_fifo(&self) -> Result<(String, Bytes)> {
        // Find the oldest key
        let oldest = self.fifo_keys.iter()
            .min_by_key(|entry| *entry.key())
            .ok_or(StorageError::KeyNotFound)?;
        
        let time = *oldest.key();
        let key = oldest.value().clone();
        
        // Remove from FIFO list
        self.fifo_keys.remove(&time);
        
        // Get and remove the value
        match self.map.remove(&key) {
            Some((k, v)) => {
                // Check if key has expired
                if let Some(expiry) = v.expiry {
                    if Instant::now() > expiry {
                        return Err(StorageError::KeyExpired);
                    }
                }
                Ok((k, v.data))
            },
            None => Err(StorageError::KeyNotFound),
        }
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        self.map.remove(key).ok_or(StorageError::KeyNotFound)?;
        // TODO: Remove from fifo_keys
        Ok(())
    }
    
    pub fn cleanup_expired(&self) -> usize {
        let now = Instant::now();
        let mut removed = 0;
        
        // Find expired keys
        let expired_keys: Vec<String> = self.map.iter()
            .filter_map(|entry| {
                if let Some(expiry) = entry.expiry {
                    if now > expiry {
                        Some(entry.key().clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        // Remove expired keys
        for key in expired_keys {
            self.map.remove(&key);
            removed += 1;
        }
        
        removed
    }

    pub fn keys(&self, pattern: &str) -> Vec<String> {
        let now = Instant::now();
        let mut keys = Vec::new();
        
        // Simple pattern matching (only supports * wildcard at the end)
        let is_wildcard = pattern.ends_with('*');
        let prefix = if is_wildcard {
            pattern[..pattern.len() - 1].to_string()
        } else {
            pattern.to_string()
        };
        
        for entry in self.map.iter() {
            let key = entry.key();
            
            // Skip expired keys
            if let Some(expiry) = entry.value().expiry {
                if now > expiry {
                    continue;
                }
            }
            
            // Match the pattern
            if pattern == "*" || 
               (is_wildcard && key.starts_with(&prefix)) || 
               (!is_wildcard && key == pattern) {
                keys.push(key.clone());
            }
        }
        
        keys
    }
}