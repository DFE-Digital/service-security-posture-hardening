use std::sync::Arc;
use std::time::SystemTime;

use rustls::client::ServerCertVerified;
use rustls::client::{HandshakeSignatureValid, ServerCertVerifier};
use rustls::Certificate;
use rustls::{ClientConfig, DigitallySignedStruct, Error, ServerName};

struct AcceptAnyCertVerifier;

impl ServerCertVerifier for AcceptAnyCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> Result<ServerCertVerified, Error> {
        // dbg!(_end_entity);
        // dbg!(_intermediates);
        // dbg!(_server_name);
        // dbg!(_scts);
        // dbg!(_ocsp_response);
        // dbg!(_now);
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &Certificate,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        // dbg!(_message);
        // dbg!(_cert);
        // dbg!(_dss);
        // LOOKS GOOD
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &Certificate,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        // dbg!(_message);
        // dbg!(_cert);
        // dbg!(_dss);
        // YUP VALID
        Ok(HandshakeSignatureValid::assertion())
    }
}

/// https://github.com/rustls/rustls/issues/127
///
pub fn crazy_x509_tls_clientconfig() -> ClientConfig {
    rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCertVerifier))
        .with_no_client_auth()
}
