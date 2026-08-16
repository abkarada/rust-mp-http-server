use std::sync::Arc;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;

/// Generate or load TLS 1.3 ServerConfig with ALPN support for HTTP/2 ("h2") and HTTP/1.1 ("http/1.1")
pub fn create_tls_config() -> Result<Arc<ServerConfig>, String> {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names)
        .map_err(|e| format!("failed to generate self-signed TLS cert: {e}"))?;

    let cert_der = cert.cert.der().to_vec();
    let key_der = cert.key_pair.serialize_der();

    let certs = vec![CertificateDer::from(cert_der)];
    let key = PrivateKeyDer::Pkcs8(key_der.into());

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("failed to build TLS ServerConfig: {e}"))?;

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_creation() {
        let config = create_tls_config();
        assert!(config.is_ok());
        let tls_config = config.unwrap();
        assert_eq!(tls_config.alpn_protocols.len(), 2);
        assert_eq!(tls_config.alpn_protocols[0], b"h2");
        assert_eq!(tls_config.alpn_protocols[1], b"http/1.1");
    }
}
