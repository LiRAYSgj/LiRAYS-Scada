use std::{path::PathBuf, sync::Arc};

use rustls_pemfile::{certs, private_key};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer},
    },
};

#[derive(Clone)]
pub struct ServerTlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

impl ServerTlsConfig {
    pub fn new(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            cert_path,
            key_path,
        }
    }
}

pub fn build_tls_acceptor(
    config: &ServerTlsConfig,
) -> Result<TlsAcceptor, Box<dyn std::error::Error + Send + Sync>> {
    let mut cert_reader = std::io::BufReader::new(std::fs::File::open(&config.cert_path)?);
    let mut key_reader = std::io::BufReader::new(std::fs::File::open(&config.key_path)?);

    let certs: Vec<CertificateDer<'static>> =
        certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

    let key: PrivateKeyDer<'static> =
        private_key(&mut key_reader)?.ok_or("no private key found in key file")?;

    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(TlsAcceptor::from(Arc::new(tls_config)))
}
