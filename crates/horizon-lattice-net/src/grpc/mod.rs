//! gRPC client support.
//!
//! This module provides gRPC client functionality using tonic, supporting:
//! - Unary calls
//! - Server streaming
//! - Client streaming
//! - Bidirectional streaming
//! - TLS support
//! - Metadata (similar to HTTP headers)
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::grpc::{GrpcChannel, GrpcMetadata};
//!
//! // Create a channel to a gRPC server
//! let channel = GrpcChannel::builder("http://localhost:50051")
//!     .connect()
//!     .await?;
//!
//! // Create metadata for authentication
//! let mut metadata = GrpcMetadata::new();
//! metadata.insert("authorization", "Bearer token");
//!
//! // Use with generated client code:
//! // let client = MyServiceClient::with_interceptor(channel.into_inner(), |req| {
//! //     req.metadata_mut().extend(metadata.clone());
//! //     Ok(req)
//! // });
//! ```
//!
//! # TLS Configuration
//!
//! ```ignore
//! use horizon_lattice_net::grpc::GrpcChannel;
//! use horizon_lattice_net::tls::TlsConfig;
//!
//! let tls_config = TlsConfig::new()
//!     .add_root_certificate_pem(include_bytes!("ca.pem"))?;
//!
//! let channel = GrpcChannel::builder("https://secure.example.com:443")
//!     .tls_config(tls_config)
//!     .connect()
//!     .await?;
//! ```
//!
//! # Code Generation
//!
//! For protocol buffer code generation, add to your `build.rs`:
//!
//! ```ignore
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     tonic_build::compile_protos("proto/service.proto")?;
//!     Ok(())
//! }
//! ```

mod channel;
mod metadata;
mod status;

pub use channel::{GrpcChannel, GrpcChannelBuilder};
pub use metadata::GrpcMetadata;
pub use status::{GrpcStatus, GrpcStatusCode};

// Re-export tonic types for advanced usage
pub use tonic;
