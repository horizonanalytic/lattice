//! GraphQL request types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A GraphQL operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    /// A query operation (read-only).
    #[default]
    Query,
    /// A mutation operation (modifies data).
    Mutation,
    /// A subscription operation (real-time updates).
    Subscription,
}

/// A GraphQL request.
///
/// Represents a GraphQL operation with optional variables and operation name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLRequest {
    /// The GraphQL query string.
    pub query: String,

    /// Optional variables for the query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Value>,

    /// Optional operation name (for documents with multiple operations).
    #[serde(skip_serializing_if = "Option::is_none", rename = "operationName")]
    pub operation_name: Option<String>,

    /// Extensions (implementation-specific metadata).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,

    /// The operation type (not serialized, used internally).
    #[serde(skip)]
    pub(crate) operation_type: OperationType,
}

impl GraphQLRequest {
    /// Create a new query request.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let request = GraphQLRequest::query(r#"
    ///     query GetUsers {
    ///         users {
    ///             id
    ///             name
    ///         }
    ///     }
    /// "#);
    /// ```
    pub fn query(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            variables: None,
            operation_name: None,
            extensions: None,
            operation_type: OperationType::Query,
        }
    }

    /// Create a new mutation request.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let request = GraphQLRequest::mutation(r#"
    ///     mutation CreateUser($name: String!) {
    ///         createUser(name: $name) {
    ///             id
    ///         }
    ///     }
    /// "#)
    /// .variable("name", "John");
    /// ```
    pub fn mutation(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            variables: None,
            operation_name: None,
            extensions: None,
            operation_type: OperationType::Mutation,
        }
    }

    /// Create a new subscription request.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let request = GraphQLRequest::subscription(r#"
    ///     subscription OnUserCreated {
    ///         userCreated {
    ///             id
    ///             name
    ///         }
    ///     }
    /// "#);
    /// ```
    pub fn subscription(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            variables: None,
            operation_name: None,
            extensions: None,
            operation_type: OperationType::Subscription,
        }
    }

    /// Create a new request from raw query string.
    ///
    /// The operation type will be inferred from the query if possible,
    /// defaulting to Query.
    pub fn new(query: impl Into<String>) -> Self {
        let query_str = query.into();
        let operation_type = Self::infer_operation_type(&query_str);
        Self {
            query: query_str,
            variables: None,
            operation_name: None,
            extensions: None,
            operation_type,
        }
    }

    /// Set a variable value.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let request = GraphQLRequest::query("...")
    ///     .variable("id", "123")
    ///     .variable("limit", 10);
    /// ```
    pub fn variable(mut self, name: impl Into<String>, value: impl Serialize) -> Self {
        let variables = self
            .variables
            .get_or_insert_with(|| Value::Object(Default::default()));
        if let Value::Object(map) = variables
            && let Ok(value) = serde_json::to_value(value) {
                map.insert(name.into(), value);
            }
        self
    }

    /// Set multiple variables from a serializable value.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let request = GraphQLRequest::query("...")
    ///     .variables(serde_json::json!({
    ///         "id": "123",
    ///         "limit": 10
    ///     }));
    /// ```
    pub fn variables(mut self, variables: impl Serialize) -> Self {
        self.variables = serde_json::to_value(variables).ok();
        self
    }

    /// Set variables from a HashMap.
    pub fn variables_map(mut self, variables: HashMap<String, Value>) -> Self {
        self.variables = Some(Value::Object(variables.into_iter().collect()));
        self
    }

    /// Set the operation name.
    ///
    /// Required when the query document contains multiple operations.
    pub fn operation_name(mut self, name: impl Into<String>) -> Self {
        self.operation_name = Some(name.into());
        self
    }

    /// Set extensions (implementation-specific metadata).
    pub fn extensions(mut self, extensions: impl Serialize) -> Self {
        self.extensions = serde_json::to_value(extensions).ok();
        self
    }

    /// Get the operation type.
    pub fn operation_type(&self) -> OperationType {
        self.operation_type
    }

    /// Check if this is a subscription.
    pub fn is_subscription(&self) -> bool {
        self.operation_type == OperationType::Subscription
    }

    /// Infer operation type from query string.
    fn infer_operation_type(query: &str) -> OperationType {
        let trimmed = query.trim_start();
        if trimmed.starts_with("subscription") || trimmed.contains("subscription ") {
            OperationType::Subscription
        } else if trimmed.starts_with("mutation") || trimmed.contains("mutation ") {
            OperationType::Mutation
        } else {
            OperationType::Query
        }
    }
}

/// Standard introspection query for schema metadata.
pub const INTROSPECTION_QUERY: &str = r#"
    query IntrospectionQuery {
        __schema {
            queryType { name }
            mutationType { name }
            subscriptionType { name }
            types {
                ...FullType
            }
            directives {
                name
                description
                locations
                args {
                    ...InputValue
                }
            }
        }
    }

    fragment FullType on __Type {
        kind
        name
        description
        fields(includeDeprecated: true) {
            name
            description
            args {
                ...InputValue
            }
            type {
                ...TypeRef
            }
            isDeprecated
            deprecationReason
        }
        inputFields {
            ...InputValue
        }
        interfaces {
            ...TypeRef
        }
        enumValues(includeDeprecated: true) {
            name
            description
            isDeprecated
            deprecationReason
        }
        possibleTypes {
            ...TypeRef
        }
    }

    fragment InputValue on __InputValue {
        name
        description
        type {
            ...TypeRef
        }
        defaultValue
    }

    fragment TypeRef on __Type {
        kind
        name
        ofType {
            kind
            name
            ofType {
                kind
                name
                ofType {
                    kind
                    name
                    ofType {
                        kind
                        name
                        ofType {
                            kind
                            name
                            ofType {
                                kind
                                name
                                ofType {
                                    kind
                                    name
                                }
                            }
                        }
                    }
                }
            }
        }
    }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_request() {
        let request = GraphQLRequest::query("{ users { id } }");
        assert_eq!(request.operation_type(), OperationType::Query);
        assert!(request.variables.is_none());
    }

    #[test]
    fn test_mutation_request() {
        let request = GraphQLRequest::mutation("mutation { createUser { id } }");
        assert_eq!(request.operation_type(), OperationType::Mutation);
    }

    #[test]
    fn test_subscription_request() {
        let request = GraphQLRequest::subscription("subscription { userCreated { id } }");
        assert_eq!(request.operation_type(), OperationType::Subscription);
        assert!(request.is_subscription());
    }

    #[test]
    fn test_variables() {
        let request = GraphQLRequest::query("query($id: ID!) { user(id: $id) { name } }")
            .variable("id", "123")
            .variable("limit", 10);

        let vars = request.variables.unwrap();
        assert_eq!(vars["id"], "123");
        assert_eq!(vars["limit"], 10);
    }

    #[test]
    fn test_operation_name() {
        let request =
            GraphQLRequest::query("query GetUser { user { id } }").operation_name("GetUser");
        assert_eq!(request.operation_name, Some("GetUser".to_string()));
    }

    #[test]
    fn test_infer_operation_type() {
        assert_eq!(
            GraphQLRequest::new("query { users }").operation_type(),
            OperationType::Query
        );
        assert_eq!(
            GraphQLRequest::new("mutation { create }").operation_type(),
            OperationType::Mutation
        );
        assert_eq!(
            GraphQLRequest::new("subscription { events }").operation_type(),
            OperationType::Subscription
        );
    }
}
