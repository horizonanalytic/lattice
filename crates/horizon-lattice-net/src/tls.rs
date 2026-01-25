//! TLS/SSL configuration types for secure connections.
//!
//! This module provides types for configuring TLS connections across all
//! networking components (HTTP, WebSocket, TCP).
//!
//! # Custom CA Certificates
//!
//! ```ignore
//! use horizon_lattice_net::tls::{Certificate, TlsConfig};
//!
//! // Load from PEM file
//! let ca_cert = Certificate::from_pem_file("/path/to/ca.crt")?;
//!
//! // Or from PEM bytes
//! let ca_cert = Certificate::from_pem(pem_bytes)?;
//!
//! // Use with HTTP client
//! let client = HttpClient::builder()
//!     .add_root_certificate(ca_cert)
//!     .build()?;
//! ```
//!
//! # Client Certificates (mTLS)
//!
//! ```ignore
//! use horizon_lattice_net::tls::Identity;
//!
//! // Load from separate PEM certificate and key files
//! let identity = Identity::from_pem_files(
//!     "/path/to/client.crt",
//!     "/path/to/client.key",
//! )?;
//!
//! // Or from a combined PEM file (cert + key in one file)
//! let identity = Identity::from_pem_combined_file("/path/to/client.pem")?;
//!
//! let client = HttpClient::builder()
//!     .identity(identity)
//!     .build()?;
//! ```
//!
//! # TLS Version Selection
//!
//! ```ignore
//! use horizon_lattice_net::tls::TlsVersion;
//!
//! let client = HttpClient::builder()
//!     .min_tls_version(TlsVersion::Tls1_3)
//!     .build()?;
//! ```

use std::io::{BufReader, Cursor};
use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore};

use crate::error::{NetworkError, Result};

/// Minimum TLS protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TlsVersion {
    /// TLS 1.2 (default minimum).
    #[default]
    Tls1_2,
    /// TLS 1.3 (most secure).
    Tls1_3,
}

impl TlsVersion {
    /// Convert to rustls protocol version.
    pub(crate) fn to_rustls_versions(self) -> Vec<&'static rustls::SupportedProtocolVersion> {
        match self {
            TlsVersion::Tls1_2 => vec![&rustls::version::TLS12, &rustls::version::TLS13],
            TlsVersion::Tls1_3 => vec![&rustls::version::TLS13],
        }
    }

    /// Convert to reqwest TLS version.
    pub(crate) fn to_reqwest_version(self) -> reqwest::tls::Version {
        match self {
            TlsVersion::Tls1_2 => reqwest::tls::Version::TLS_1_2,
            TlsVersion::Tls1_3 => reqwest::tls::Version::TLS_1_3,
        }
    }
}

/// ALPN (Application-Layer Protocol Negotiation) protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlpnProtocol {
    /// HTTP/1.1
    Http1,
    /// HTTP/2
    H2,
    /// Custom protocol identifier.
    Custom(Vec<u8>),
}

impl AlpnProtocol {
    /// Get the protocol identifier bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AlpnProtocol::Http1 => b"http/1.1",
            AlpnProtocol::H2 => b"h2",
            AlpnProtocol::Custom(bytes) => bytes,
        }
    }

    /// Convert to owned bytes.
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

/// A TLS certificate for server verification.
///
/// This can be a CA certificate to add to the trust store, or a certificate
/// chain for client authentication.
#[derive(Clone)]
pub struct Certificate {
    der_certs: Vec<CertificateDer<'static>>,
}

impl std::fmt::Debug for Certificate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Certificate")
            .field("cert_count", &self.der_certs.len())
            .finish()
    }
}

impl Certificate {
    /// Load a certificate from PEM-encoded bytes.
    ///
    /// This can contain multiple certificates (a certificate chain).
    pub fn from_pem(pem_data: impl AsRef<[u8]>) -> Result<Self> {
        let mut reader = BufReader::new(Cursor::new(pem_data.as_ref()));
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| NetworkError::Tls(format!("Failed to parse PEM certificate: {}", e)))?;

        if certs.is_empty() {
            return Err(NetworkError::Tls(
                "No certificates found in PEM data".to_string(),
            ));
        }

        Ok(Self { der_certs: certs })
    }

    /// Load a certificate from a PEM-encoded file.
    pub fn from_pem_file(path: impl AsRef<Path>) -> Result<Self> {
        let pem_data = std::fs::read(path.as_ref()).map_err(|e| {
            NetworkError::Tls(format!(
                "Failed to read certificate file '{}': {}",
                path.as_ref().display(),
                e
            ))
        })?;
        Self::from_pem(pem_data)
    }

    /// Load a certificate from DER-encoded bytes.
    pub fn from_der(der_data: impl Into<Vec<u8>>) -> Self {
        Self {
            der_certs: vec![CertificateDer::from(der_data.into())],
        }
    }

    /// Load a certificate from a DER-encoded file.
    pub fn from_der_file(path: impl AsRef<Path>) -> Result<Self> {
        let der_data = std::fs::read(path.as_ref()).map_err(|e| {
            NetworkError::Tls(format!(
                "Failed to read certificate file '{}': {}",
                path.as_ref().display(),
                e
            ))
        })?;
        Ok(Self::from_der(der_data))
    }

    /// Load a certificate bundle from PEM-encoded bytes.
    ///
    /// This is useful for loading a CA bundle file containing multiple certificates.
    pub fn from_pem_bundle(pem_data: impl AsRef<[u8]>) -> Result<Vec<Self>> {
        let mut reader = BufReader::new(Cursor::new(pem_data.as_ref()));
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| NetworkError::Tls(format!("Failed to parse PEM bundle: {}", e)))?;

        Ok(certs
            .into_iter()
            .map(|cert| Certificate {
                der_certs: vec![cert],
            })
            .collect())
    }

    /// Load a certificate bundle from a PEM-encoded file.
    pub fn from_pem_bundle_file(path: impl AsRef<Path>) -> Result<Vec<Self>> {
        let pem_data = std::fs::read(path.as_ref()).map_err(|e| {
            NetworkError::Tls(format!(
                "Failed to read certificate bundle '{}': {}",
                path.as_ref().display(),
                e
            ))
        })?;
        Self::from_pem_bundle(pem_data)
    }

    /// Get the DER-encoded certificates.
    pub(crate) fn der_certs(&self) -> &[CertificateDer<'static>] {
        &self.der_certs
    }

    /// Convert to reqwest Certificate for use with the HTTP client.
    pub(crate) fn to_reqwest_certificates(&self) -> Vec<reqwest::Certificate> {
        self.der_certs
            .iter()
            .filter_map(|cert| reqwest::Certificate::from_der(cert.as_ref()).ok())
            .collect()
    }
}

/// Client identity for mutual TLS (mTLS) authentication.
///
/// An identity consists of a client certificate and its corresponding private key.
pub struct Identity {
    cert_chain: Vec<CertificateDer<'static>>,
    private_key: PrivateKeyDer<'static>,
}

impl Clone for Identity {
    fn clone(&self) -> Self {
        Self {
            cert_chain: self.cert_chain.clone(),
            private_key: self.private_key.clone_key(),
        }
    }
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("cert_count", &self.cert_chain.len())
            .field("has_key", &true)
            .finish()
    }
}

impl Identity {
    /// Create an identity from PEM-encoded certificate and key bytes.
    ///
    /// The certificate can be a single certificate or a chain (with the client
    /// certificate first, followed by intermediate certificates).
    pub fn from_pem(cert_pem: impl AsRef<[u8]>, key_pem: impl AsRef<[u8]>) -> Result<Self> {
        // Parse certificate chain
        let mut cert_reader = BufReader::new(Cursor::new(cert_pem.as_ref()));
        let cert_chain: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| NetworkError::Tls(format!("Failed to parse certificate PEM: {}", e)))?;

        if cert_chain.is_empty() {
            return Err(NetworkError::Tls(
                "No certificates found in PEM data".to_string(),
            ));
        }

        // Parse private key (try different key formats)
        let mut key_reader = BufReader::new(Cursor::new(key_pem.as_ref()));
        let private_key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| NetworkError::Tls(format!("Failed to parse private key PEM: {}", e)))?
            .ok_or_else(|| NetworkError::Tls("No private key found in PEM data".to_string()))?;

        Ok(Self {
            cert_chain,
            private_key,
        })
    }

    /// Create an identity from PEM-encoded files.
    pub fn from_pem_files(
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let cert_pem = std::fs::read(cert_path.as_ref()).map_err(|e| {
            NetworkError::Tls(format!(
                "Failed to read certificate file '{}': {}",
                cert_path.as_ref().display(),
                e
            ))
        })?;

        let key_pem = std::fs::read(key_path.as_ref()).map_err(|e| {
            NetworkError::Tls(format!(
                "Failed to read key file '{}': {}",
                key_path.as_ref().display(),
                e
            ))
        })?;

        Self::from_pem(cert_pem, key_pem)
    }

    /// Create an identity from a combined PEM file containing both certificate and key.
    ///
    /// This is the format expected by reqwest's `Identity::from_pem()`.
    pub fn from_pem_combined(pem_data: impl AsRef<[u8]>) -> Result<Self> {
        let pem = pem_data.as_ref();

        // Parse certificate chain
        let mut cert_reader = BufReader::new(Cursor::new(pem));
        let cert_chain: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| NetworkError::Tls(format!("Failed to parse certificate PEM: {}", e)))?;

        if cert_chain.is_empty() {
            return Err(NetworkError::Tls(
                "No certificates found in PEM data".to_string(),
            ));
        }

        // Parse private key
        let mut key_reader = BufReader::new(Cursor::new(pem));
        let private_key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| NetworkError::Tls(format!("Failed to parse private key PEM: {}", e)))?
            .ok_or_else(|| NetworkError::Tls("No private key found in PEM data".to_string()))?;

        Ok(Self {
            cert_chain,
            private_key,
        })
    }

    /// Create an identity from a combined PEM file.
    pub fn from_pem_combined_file(path: impl AsRef<Path>) -> Result<Self> {
        let pem_data = std::fs::read(path.as_ref()).map_err(|e| {
            NetworkError::Tls(format!(
                "Failed to read PEM file '{}': {}",
                path.as_ref().display(),
                e
            ))
        })?;
        Self::from_pem_combined(pem_data)
    }

    /// Get the certificate chain.
    pub(crate) fn cert_chain(&self) -> &[CertificateDer<'static>] {
        &self.cert_chain
    }

    /// Get the private key.
    pub(crate) fn private_key(&self) -> &PrivateKeyDer<'static> {
        &self.private_key
    }

    /// Convert to reqwest Identity for use with the HTTP client.
    ///
    /// This creates a combined PEM format that reqwest expects.
    pub(crate) fn to_reqwest_identity(&self) -> Result<reqwest::Identity> {
        // Reqwest expects a combined PEM with cert + key
        // We need to re-encode as PEM
        use std::io::Write;
        let mut pem_buf = Vec::new();

        // Write certificates
        for cert in &self.cert_chain {
            writeln!(pem_buf, "-----BEGIN CERTIFICATE-----").unwrap();
            let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, cert.as_ref());
            for chunk in b64.as_bytes().chunks(64) {
                pem_buf.extend_from_slice(chunk);
                pem_buf.push(b'\n');
            }
            writeln!(pem_buf, "-----END CERTIFICATE-----").unwrap();
        }

        // Write private key
        let (label, key_bytes) = match &self.private_key {
            PrivateKeyDer::Pkcs1(key) => ("RSA PRIVATE KEY", key.secret_pkcs1_der()),
            PrivateKeyDer::Pkcs8(key) => ("PRIVATE KEY", key.secret_pkcs8_der()),
            PrivateKeyDer::Sec1(key) => ("EC PRIVATE KEY", key.secret_sec1_der()),
            _ => return Err(NetworkError::Tls("Unknown private key format".to_string())),
        };

        writeln!(pem_buf, "-----BEGIN {}-----", label).unwrap();
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key_bytes);
        for chunk in b64.as_bytes().chunks(64) {
            pem_buf.extend_from_slice(chunk);
            pem_buf.push(b'\n');
        }
        writeln!(pem_buf, "-----END {}-----", label).unwrap();

        reqwest::Identity::from_pem(&pem_buf)
            .map_err(|e| NetworkError::Tls(format!("Failed to create identity: {}", e)))
    }
}

/// Complete TLS configuration for a connection.
///
/// This combines all TLS settings: CA certificates, client identity, version,
/// and ALPN protocols.
#[derive(Debug, Clone, Default)]
pub struct TlsConfig {
    /// Additional root certificates to trust.
    pub root_certificates: Vec<Certificate>,
    /// Whether to use only the provided root certificates (no system roots).
    pub use_only_custom_roots: bool,
    /// Client identity for mutual TLS.
    pub identity: Option<Identity>,
    /// Minimum TLS version.
    pub min_version: TlsVersion,
    /// ALPN protocols to advertise.
    pub alpn_protocols: Vec<AlpnProtocol>,
    /// Accept invalid/self-signed certificates (DANGEROUS - testing only).
    pub danger_accept_invalid_certs: bool,
    /// Accept invalid hostnames (DANGEROUS - testing only).
    pub danger_accept_invalid_hostnames: bool,
}

impl TlsConfig {
    /// Create a new TLS configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a root certificate to trust.
    pub fn add_root_certificate(mut self, cert: Certificate) -> Self {
        self.root_certificates.push(cert);
        self
    }

    /// Add multiple root certificates.
    pub fn add_root_certificates(mut self, certs: impl IntoIterator<Item = Certificate>) -> Self {
        self.root_certificates.extend(certs);
        self
    }

    /// Use only custom root certificates (disable system roots).
    pub fn use_only_custom_roots(mut self) -> Self {
        self.use_only_custom_roots = true;
        self
    }

    /// Set the client identity for mutual TLS.
    pub fn identity(mut self, identity: Identity) -> Self {
        self.identity = Some(identity);
        self
    }

    /// Set the minimum TLS version.
    pub fn min_version(mut self, version: TlsVersion) -> Self {
        self.min_version = version;
        self
    }

    /// Add an ALPN protocol.
    pub fn alpn_protocol(mut self, protocol: AlpnProtocol) -> Self {
        self.alpn_protocols.push(protocol);
        self
    }

    /// Set ALPN protocols.
    pub fn alpn_protocols(mut self, protocols: impl IntoIterator<Item = AlpnProtocol>) -> Self {
        self.alpn_protocols = protocols.into_iter().collect();
        self
    }

    /// Accept invalid certificates (DANGEROUS - for testing only).
    ///
    /// # Warning
    ///
    /// This disables certificate verification and makes the connection
    /// vulnerable to man-in-the-middle attacks.
    pub fn danger_accept_invalid_certs(mut self) -> Self {
        self.danger_accept_invalid_certs = true;
        self
    }

    /// Accept invalid hostnames (DANGEROUS - for testing only).
    ///
    /// # Warning
    ///
    /// This disables hostname verification and makes the connection
    /// vulnerable to man-in-the-middle attacks.
    pub fn danger_accept_invalid_hostnames(mut self) -> Self {
        self.danger_accept_invalid_hostnames = true;
        self
    }

    /// Check if this configuration has any custom settings.
    pub fn is_default(&self) -> bool {
        self.root_certificates.is_empty()
            && !self.use_only_custom_roots
            && self.identity.is_none()
            && self.min_version == TlsVersion::Tls1_2
            && self.alpn_protocols.is_empty()
            && !self.danger_accept_invalid_certs
            && !self.danger_accept_invalid_hostnames
    }

    /// Build a rustls ClientConfig from this TLS configuration.
    ///
    /// This is used by WebSocket and TCP clients.
    pub fn build_rustls_config(&self) -> Result<Arc<ClientConfig>> {
        // Build root certificate store
        let root_store = self.build_root_store()?;

        // Build client config
        let versions = self.min_version.to_rustls_versions();

        let builder = ClientConfig::builder_with_protocol_versions(&versions)
            .with_root_certificates(root_store);

        let mut config = if let Some(ref identity) = self.identity {
            builder
                .with_client_auth_cert(
                    identity.cert_chain().to_vec(),
                    identity.private_key().clone_key(),
                )
                .map_err(|e| NetworkError::Tls(format!("Invalid client certificate: {}", e)))?
        } else {
            builder.with_no_client_auth()
        };

        // Set ALPN protocols
        if !self.alpn_protocols.is_empty() {
            config.alpn_protocols = self.alpn_protocols.iter().map(|p| p.to_vec()).collect();
        }

        Ok(Arc::new(config))
    }

    /// Build a root certificate store.
    fn build_root_store(&self) -> Result<RootCertStore> {
        let mut root_store = RootCertStore::empty();

        // Add system roots unless using only custom roots
        if !self.use_only_custom_roots {
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }

        // Add custom root certificates
        for cert in &self.root_certificates {
            for der_cert in cert.der_certs() {
                root_store.add(der_cert.clone()).map_err(|e| {
                    NetworkError::Tls(format!("Failed to add root certificate: {}", e))
                })?;
            }
        }

        if root_store.is_empty() {
            return Err(NetworkError::Tls(
                "No root certificates available. Either add custom certificates or \
                 don't use use_only_custom_roots()"
                    .to_string(),
            ));
        }

        Ok(root_store)
    }
}

/// A dangerous certificate verifier that accepts all certificates.
///
/// This is used when `danger_accept_invalid_certs` is enabled.
#[derive(Debug)]
pub(crate) struct DangerousVerifier;

impl rustls::client::danger::ServerCertVerifier for DangerousVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

impl TlsConfig {
    /// Build a rustls ClientConfig that accepts invalid certificates.
    ///
    /// This is only used when `danger_accept_invalid_certs` is true.
    pub fn build_dangerous_rustls_config(&self) -> Result<Arc<ClientConfig>> {
        let versions = self.min_version.to_rustls_versions();

        let builder = ClientConfig::builder_with_protocol_versions(&versions)
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(DangerousVerifier));

        let mut config = if let Some(ref identity) = self.identity {
            builder
                .with_client_auth_cert(
                    identity.cert_chain().to_vec(),
                    identity.private_key().clone_key(),
                )
                .map_err(|e| NetworkError::Tls(format!("Invalid client certificate: {}", e)))?
        } else {
            builder.with_no_client_auth()
        };

        // Set ALPN protocols
        if !self.alpn_protocols.is_empty() {
            config.alpn_protocols = self.alpn_protocols.iter().map(|p| p.to_vec()).collect();
        }

        Ok(Arc::new(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Install the ring crypto provider for tests.
    fn install_crypto_provider() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    #[test]
    fn test_tls_version_default() {
        assert_eq!(TlsVersion::default(), TlsVersion::Tls1_2);
    }

    #[test]
    fn test_tls_config_is_default() {
        let config = TlsConfig::new();
        assert!(config.is_default());

        let config = TlsConfig::new().min_version(TlsVersion::Tls1_3);
        assert!(!config.is_default());
    }

    #[test]
    fn test_alpn_protocol_bytes() {
        assert_eq!(AlpnProtocol::Http1.as_bytes(), b"http/1.1");
        assert_eq!(AlpnProtocol::H2.as_bytes(), b"h2");
        assert_eq!(
            AlpnProtocol::Custom(b"custom".to_vec()).as_bytes(),
            b"custom"
        );
    }

    #[test]
    fn test_certificate_from_pem() {
        // A simple self-signed certificate for testing
        let pem = r#"-----BEGIN CERTIFICATE-----
MIIBkTCB+wIJAKHBfpegE3jEMA0GCSqGSIb3DQEBCwUAMBExDzANBgNVBAMMBnRl
c3RjYTAeFw0yMzAxMDEwMDAwMDBaFw0yNDAxMDEwMDAwMDBaMBExDzANBgNVBAMM
BnRlc3RjYTBcMA0GCSqGSIb3DQEBAQUAA0sAMEgCQQC7o96HtiK7onnPevKSE2LL
oSXwnmfYwZPV2bvfGS18lK8F+DL+42IjT3ucMXnLBhzNCLNKE8yCVK6LPlsvpNlX
AgMBAAGjUzBRMB0GA1UdDgQWBBQgHGHqPcVi1N4CG7IxDJaFMvP6XTAfBgNVHSME
GDAWgBQgHGHqPcVi1N4CG7IxDJaFMvP6XTAPBgNVHRMBAf8EBTADAQH/MA0GCSqG
SIb3DQEBCwUAA0EAGLJHfg9dS/T39L6VQLJeZcpH7mY8vKaM9dM/Zn3HMhfc0Yjv
3hxMPmPGjjpQ9JKaLI0Rq7n5oEUP+xluoAAfrQ==
-----END CERTIFICATE-----"#;

        let cert = Certificate::from_pem(pem).unwrap();
        assert_eq!(cert.der_certs().len(), 1);
    }

    #[test]
    fn test_certificate_from_der() {
        // Minimal DER certificate (just testing the wrapper works)
        let der = vec![0x30, 0x03, 0x02, 0x01, 0x00];
        let cert = Certificate::from_der(der);
        assert_eq!(cert.der_certs().len(), 1);
    }

    #[test]
    fn test_tls_config_builder() {
        let config = TlsConfig::new()
            .min_version(TlsVersion::Tls1_3)
            .alpn_protocol(AlpnProtocol::H2)
            .alpn_protocol(AlpnProtocol::Http1);

        assert_eq!(config.min_version, TlsVersion::Tls1_3);
        assert_eq!(config.alpn_protocols.len(), 2);
    }

    #[test]
    fn test_build_rustls_config_with_system_roots() {
        install_crypto_provider();
        // Default config should use system roots
        let config = TlsConfig::new();
        let rustls_config = config.build_rustls_config();
        assert!(rustls_config.is_ok());
    }

    #[test]
    fn test_build_rustls_config_only_custom_roots_without_certs_fails() {
        install_crypto_provider();
        // Using only custom roots without providing any should fail
        let config = TlsConfig::new().use_only_custom_roots();
        let result = config.build_rustls_config();
        assert!(result.is_err());
    }

    #[test]
    fn test_build_dangerous_rustls_config() {
        install_crypto_provider();
        // Dangerous config should build successfully
        let config = TlsConfig::new().danger_accept_invalid_certs();
        let rustls_config = config.build_dangerous_rustls_config();
        assert!(rustls_config.is_ok());
    }

    #[test]
    fn test_tls_config_with_custom_root_cert() {
        // A simple self-signed certificate for testing (just PEM parsing)
        let pem = r#"-----BEGIN CERTIFICATE-----
MIIBkTCB+wIJAKHBfpegE3jEMA0GCSqGSIb3DQEBCwUAMBExDzANBgNVBAMMBnRl
c3RjYTAeFw0yMzAxMDEwMDAwMDBaFw0yNDAxMDEwMDAwMDBaMBExDzANBgNVBAMM
BnRlc3RjYTBcMA0GCSqGSIb3DQEBAQUAA0sAMEgCQQC7o96HtiK7onnPevKSE2LL
oSXwnmfYwZPV2bvfGS18lK8F+DL+42IjT3ucMXnLBhzNCLNKE8yCVK6LPlsvpNlX
AgMBAAGjUzBRMB0GA1UdDgQWBBQgHGHqPcVi1N4CG7IxDJaFMvP6XTAfBgNVHSME
GDAWgBQgHGHqPcVi1N4CG7IxDJaFMvP6XTAPBgNVHRMBAf8EBTADAQH/MA0GCSqG
SIb3DQEBCwUAA0EAGLJHfg9dS/T39L6VQLJeZcpH7mY8vKaM9dM/Zn3HMhfc0Yjv
3hxMPmPGjjpQ9JKaLI0Rq7n5oEUP+xluoAAfrQ==
-----END CERTIFICATE-----"#;

        let cert = Certificate::from_pem(pem).unwrap();
        // Verify certificate was parsed
        assert_eq!(cert.der_certs().len(), 1);

        // Adding a custom cert to the config should work
        let config = TlsConfig::new().add_root_certificate(cert);
        assert_eq!(config.root_certificates.len(), 1);
        assert!(!config.is_default());
    }

    #[test]
    fn test_tls_version_to_rustls_versions() {
        let tls12 = TlsVersion::Tls1_2.to_rustls_versions();
        assert_eq!(tls12.len(), 2); // TLS 1.2 and 1.3

        let tls13 = TlsVersion::Tls1_3.to_rustls_versions();
        assert_eq!(tls13.len(), 1); // Only TLS 1.3
    }
}
