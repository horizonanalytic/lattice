//! GraphQL client for queries, mutations, and subscriptions.
//!
//! This module provides a GraphQL client that supports:
//! - Query and mutation execution
//! - Variables via JSON
//! - Subscriptions over WebSocket (graphql-transport-ws protocol)
//! - Schema introspection
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::graphql::{GraphQLClient, GraphQLRequest};
//!
//! // Create a client
//! let client = GraphQLClient::new("https://api.example.com/graphql");
//!
//! // Execute a query
//! let request = GraphQLRequest::query(r#"
//!     query GetUser($id: ID!) {
//!         user(id: $id) {
//!             id
//!             name
//!             email
//!         }
//!     }
//! "#)
//! .variable("id", "123");
//!
//! let response = client.execute(request).await?;
//! let user: User = response.data()?;
//!
//! // Execute a mutation
//! let mutation = GraphQLRequest::mutation(r#"
//!     mutation CreateUser($input: CreateUserInput!) {
//!         createUser(input: $input) {
//!             id
//!             name
//!         }
//!     }
//! "#)
//! .variable("input", serde_json::json!({
//!     "name": "John",
//!     "email": "john@example.com"
//! }));
//!
//! let response = client.execute(mutation).await?;
//! ```
//!
//! # Subscriptions
//!
//! ```ignore
//! use horizon_lattice_net::graphql::{GraphQLClient, GraphQLRequest};
//!
//! let client = GraphQLClient::new("https://api.example.com/graphql")
//!     .websocket_url("wss://api.example.com/graphql");
//!
//! // Subscribe to events
//! let subscription = GraphQLRequest::subscription(r#"
//!     subscription OnMessage {
//!         messageReceived {
//!             id
//!             content
//!         }
//!     }
//! "#);
//!
//! let mut stream = client.subscribe(subscription).await?;
//! while let Some(response) = stream.next().await {
//!     let message: Message = response?.data()?;
//!     println!("Received: {:?}", message);
//! }
//! ```

mod client;
mod request;
mod response;
mod subscription;

pub use client::{GraphQLClient, GraphQLClientBuilder};
pub use request::GraphQLRequest;
pub use response::{GraphQLError, GraphQLResponse};
pub use subscription::{SubscriptionMessage, SubscriptionStream};
