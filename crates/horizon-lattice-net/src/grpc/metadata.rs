//! gRPC metadata (headers).

use std::collections::HashMap;
use std::str::FromStr;

use tonic::metadata::{Ascii, MetadataKey, MetadataMap, MetadataValue};

use crate::error::{NetworkError, Result};

/// gRPC metadata for requests and responses.
///
/// Metadata in gRPC is similar to HTTP headers and is used to pass
/// additional information about the call.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_net::grpc::GrpcMetadata;
///
/// let mut metadata = GrpcMetadata::new();
/// metadata.insert("authorization", "Bearer token")?;
/// metadata.insert("x-request-id", "12345")?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct GrpcMetadata {
    inner: MetadataMap,
}

impl GrpcMetadata {
    /// Create empty metadata.
    pub fn new() -> Self {
        Self {
            inner: MetadataMap::new(),
        }
    }

    /// Insert a metadata entry.
    ///
    /// The key must be a valid ASCII header name.
    /// The value must be valid ASCII (use `insert_binary` for binary values).
    pub fn insert(&mut self, key: &str, value: &str) -> Result<()> {
        let key = MetadataKey::from_str(key)
            .map_err(|e| NetworkError::InvalidHeader(format!("Invalid metadata key: {}", e)))?;
        let value: MetadataValue<Ascii> = value
            .parse()
            .map_err(|e| NetworkError::InvalidHeader(format!("Invalid metadata value: {}", e)))?;
        self.inner.insert(key, value);
        Ok(())
    }

    /// Insert a binary metadata entry.
    ///
    /// Binary metadata keys must end with "-bin".
    pub fn insert_binary(&mut self, key: &str, value: &[u8]) -> Result<()> {
        let key = if key.ends_with("-bin") {
            key.to_string()
        } else {
            format!("{}-bin", key)
        };

        let key = MetadataKey::from_str(&key)
            .map_err(|e| NetworkError::InvalidHeader(format!("Invalid metadata key: {}", e)))?;
        let value = MetadataValue::from_bytes(value);
        self.inner.insert_bin(key, value);
        Ok(())
    }

    /// Get a metadata value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        let key = MetadataKey::from_str(key).ok()?;
        self.inner.get(&key).and_then(|v| v.to_str().ok())
    }

    /// Get a binary metadata value by key.
    pub fn get_binary(&self, key: &str) -> Option<&[u8]> {
        let key = if key.ends_with("-bin") {
            key.to_string()
        } else {
            format!("{}-bin", key)
        };
        let key = MetadataKey::from_str(&key).ok()?;
        self.inner.get_bin(&key).map(|v| v.as_encoded_bytes())
    }

    /// Remove a metadata entry.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        let key = MetadataKey::<Ascii>::from_str(key).ok()?;
        self.inner.remove(&key).and_then(|v| v.to_str().ok().map(|s| s.to_string()))
    }

    /// Check if a key exists.
    pub fn contains_key(&self, key: &str) -> bool {
        MetadataKey::<Ascii>::from_str(key)
            .map(|k| self.inner.contains_key(&k))
            .unwrap_or(false)
    }

    /// Check if the metadata is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        // MetadataMap doesn't have a clear method, so we replace it
        self.inner = MetadataMap::new();
    }

    /// Iterate over ASCII entries.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner
            .iter()
            .filter_map(|entry| {
                if let tonic::metadata::KeyAndValueRef::Ascii(key, value) = entry {
                    Some((key.as_str(), value.to_str().ok()?))
                } else {
                    None
                }
            })
    }

    /// Get the underlying tonic MetadataMap.
    pub fn into_inner(self) -> MetadataMap {
        self.inner
    }

    /// Get a reference to the underlying MetadataMap.
    pub fn inner(&self) -> &MetadataMap {
        &self.inner
    }

    /// Get a mutable reference to the underlying MetadataMap.
    pub fn inner_mut(&mut self) -> &mut MetadataMap {
        &mut self.inner
    }

    /// Create from a HashMap.
    pub fn from_map(map: HashMap<String, String>) -> Result<Self> {
        let mut metadata = Self::new();
        for (key, value) in map {
            metadata.insert(&key, &value)?;
        }
        Ok(metadata)
    }

    /// Convert to a HashMap (ASCII values only).
    pub fn to_map(&self) -> HashMap<String, String> {
        self.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
}

impl From<MetadataMap> for GrpcMetadata {
    fn from(inner: MetadataMap) -> Self {
        Self { inner }
    }
}

impl From<GrpcMetadata> for MetadataMap {
    fn from(metadata: GrpcMetadata) -> Self {
        metadata.inner
    }
}

/// Extension trait to apply metadata to gRPC requests.
pub trait WithMetadata {
    /// Apply metadata to this request.
    fn with_metadata(self, metadata: &GrpcMetadata) -> Self;
}

impl<T> WithMetadata for tonic::Request<T> {
    fn with_metadata(mut self, metadata: &GrpcMetadata) -> Self {
        for entry in metadata.inner.iter() {
            match entry {
                tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                    self.metadata_mut().insert(key.clone(), value.clone());
                }
                tonic::metadata::KeyAndValueRef::Binary(key, value) => {
                    self.metadata_mut().insert_bin(key.clone(), value.clone());
                }
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_insert_get() {
        let mut metadata = GrpcMetadata::new();
        metadata.insert("authorization", "Bearer token").unwrap();

        assert_eq!(metadata.get("authorization"), Some("Bearer token"));
        assert!(metadata.contains_key("authorization"));
    }

    #[test]
    fn test_metadata_remove() {
        let mut metadata = GrpcMetadata::new();
        metadata.insert("key", "value").unwrap();

        let removed = metadata.remove("key");
        assert_eq!(removed, Some("value".to_string()));
        assert!(!metadata.contains_key("key"));
    }

    #[test]
    fn test_metadata_from_map() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        map.insert("key2".to_string(), "value2".to_string());

        let metadata = GrpcMetadata::from_map(map).unwrap();
        assert_eq!(metadata.get("key1"), Some("value1"));
        assert_eq!(metadata.get("key2"), Some("value2"));
    }

    #[test]
    fn test_metadata_iter() {
        let mut metadata = GrpcMetadata::new();
        metadata.insert("key1", "value1").unwrap();
        metadata.insert("key2", "value2").unwrap();

        let map = metadata.to_map();
        assert_eq!(map.len(), 2);
    }
}
