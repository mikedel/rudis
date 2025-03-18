#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::time::Duration;
    
    #[test]
    fn test_storage_set_get() {
        let storage = Storage::new();
        let key = "test_key".to_string();
        let value = Bytes::from("test_value".as_bytes().to_vec());
        
        assert!(storage.set(key.clone(), value.clone(), None).is_ok());
        
        let result = storage.get(&key);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), value);
    }
    
    #[test]
    fn test_storage_expiration() {
        let storage = Storage::new();
        let key = "expiring_key".to_string();
        let value = Bytes::from("test_value".as_bytes().to_vec());
        
        // Set with very short TTL
        assert!(storage.set(key.clone(), value, Some(Duration::from_millis(10))).is_ok());
        
        // Should be available immediately
        assert!(storage.get(&key).is_ok());
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));
        
        // Should be expired now
        assert!(matches!(storage.get(&key), Err(StorageError::KeyExpired)));
    }
    
    #[test]
    fn test_storage_fifo() {
        let storage = Storage::new();
        
        // Add multiple keys
        for i in 0..5 {
            let key = format!("key_{}", i);
            let value = Bytes::from(format!("value_{}", i).as_bytes().to_vec());
            assert!(storage.set(key, value, None).is_ok());
        }
        
        // Pop them in FIFO order
        for i in 0..5 {
            let result = storage.pop_fifo();
            assert!(result.is_ok());
            let (key, value) = result.unwrap();
            assert_eq!(key, format!("key_{}", i));
            assert_eq!(value, Bytes::from(format!("value_{}", i).as_bytes().to_vec()));
        }
        
        // Queue should be empty now
        assert!(matches!(storage.pop_fifo(), Err(StorageError::KeyNotFound)));
    }
}