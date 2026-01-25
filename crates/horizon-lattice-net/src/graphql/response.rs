//! GraphQL response types.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use crate::error::NetworkError;

/// A GraphQL error returned by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    /// The error message.
    pub message: String,

    /// Locations in the document where the error occurred.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<GraphQLLocation>,

    /// Path to the field that caused the error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<PathSegment>>,

    /// Additional error metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,
}

impl fmt::Display for GraphQLError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref path) = self.path {
            write!(f, " (at ")?;
            for (i, segment) in path.iter().enumerate() {
                if i > 0 {
                    write!(f, ".")?;
                }
                match segment {
                    PathSegment::Field(name) => write!(f, "{}", name)?,
                    PathSegment::Index(idx) => write!(f, "[{}]", idx)?,
                }
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl std::error::Error for GraphQLError {}

/// A location in a GraphQL document.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GraphQLLocation {
    /// Line number (1-indexed).
    pub line: u32,
    /// Column number (1-indexed).
    pub column: u32,
}

/// A segment in an error path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PathSegment {
    /// A field name.
    Field(String),
    /// An array index.
    Index(usize),
}

/// A GraphQL response from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse {
    /// The data returned by the operation.
    #[serde(default)]
    pub data: Option<Value>,

    /// Errors that occurred during execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<GraphQLError>,

    /// Additional response metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,
}

impl GraphQLResponse {
    /// Check if the response contains errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if the response was successful (has data and no errors).
    pub fn is_success(&self) -> bool {
        self.data.is_some() && self.errors.is_empty()
    }

    /// Get the first error, if any.
    pub fn first_error(&self) -> Option<&GraphQLError> {
        self.errors.first()
    }

    /// Get all errors as a combined message.
    pub fn error_message(&self) -> Option<String> {
        if self.errors.is_empty() {
            None
        } else {
            Some(
                self.errors
                    .iter()
                    .map(|e| e.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        }
    }

    /// Parse the data as a specific type.
    ///
    /// Returns an error if the response has errors or if parsing fails.
    pub fn data<T: DeserializeOwned>(&self) -> Result<T, NetworkError> {
        if let Some(ref errors) = self.error_message() {
            return Err(NetworkError::Request(format!("GraphQL error: {}", errors)));
        }

        match &self.data {
            Some(data) => serde_json::from_value(data.clone()).map_err(|e| {
                NetworkError::Json(format!("Failed to deserialize GraphQL response: {}", e))
            }),
            None => Err(NetworkError::InvalidBody("No data in GraphQL response".into())),
        }
    }

    /// Parse a specific field from the data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // For a response like: { "data": { "user": { "id": "1", "name": "John" } } }
    /// let user: User = response.field("user")?;
    /// ```
    pub fn field<T: DeserializeOwned>(&self, field: &str) -> Result<T, NetworkError> {
        if let Some(ref errors) = self.error_message() {
            return Err(NetworkError::Request(format!("GraphQL error: {}", errors)));
        }

        match &self.data {
            Some(Value::Object(data)) => {
                let field_value = data.get(field).ok_or_else(|| {
                    NetworkError::InvalidBody(format!("Field '{}' not found in response", field))
                })?;
                serde_json::from_value(field_value.clone()).map_err(|e| {
                    NetworkError::Json(format!(
                        "Failed to deserialize field '{}': {}",
                        field, e
                    ))
                })
            }
            Some(_) => Err(NetworkError::InvalidBody(
                "Response data is not an object".into(),
            )),
            None => Err(NetworkError::InvalidBody("No data in GraphQL response".into())),
        }
    }

    /// Get raw data as Value without parsing.
    pub fn raw_data(&self) -> Option<&Value> {
        self.data.as_ref()
    }

    /// Convert errors to a Result.
    ///
    /// Returns `Ok(self)` if no errors, or `Err` with the first error.
    pub fn into_result(self) -> Result<Self, NetworkError> {
        if let Some(msg) = self.error_message() {
            Err(NetworkError::Request(format!("GraphQL error: {}", msg)))
        } else {
            Ok(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_successful_response() {
        let response = GraphQLResponse {
            data: Some(json!({"user": {"id": "1", "name": "John"}})),
            errors: vec![],
            extensions: None,
        };

        assert!(response.is_success());
        assert!(!response.has_errors());
    }

    #[test]
    fn test_error_response() {
        let response = GraphQLResponse {
            data: None,
            errors: vec![GraphQLError {
                message: "User not found".to_string(),
                locations: vec![GraphQLLocation { line: 1, column: 1 }],
                path: Some(vec![PathSegment::Field("user".to_string())]),
                extensions: None,
            }],
            extensions: None,
        };

        assert!(!response.is_success());
        assert!(response.has_errors());
        assert_eq!(response.error_message(), Some("User not found".to_string()));
    }

    #[test]
    fn test_parse_field() {
        let response = GraphQLResponse {
            data: Some(json!({"user": {"id": "1", "name": "John"}})),
            errors: vec![],
            extensions: None,
        };

        #[derive(Debug, Deserialize, PartialEq)]
        struct User {
            id: String,
            name: String,
        }

        let user: User = response.field("user").unwrap();
        assert_eq!(user.id, "1");
        assert_eq!(user.name, "John");
    }

    #[test]
    fn test_partial_response() {
        // GraphQL can return partial data with errors
        let response = GraphQLResponse {
            data: Some(json!({"user": null})),
            errors: vec![GraphQLError {
                message: "Permission denied".to_string(),
                locations: vec![],
                path: Some(vec![PathSegment::Field("user".to_string())]),
                extensions: None,
            }],
            extensions: None,
        };

        assert!(response.has_errors());
        // data() should fail because there are errors
        assert!(response.data::<Value>().is_err());
    }
}
