//! Types to control access to HTTP API

use bui_backend_types::AccessToken;
use std::net::SocketAddr;

#[derive(Clone, Debug)]
struct JwtSecret(Vec<u8>);

/// Data required to specify all auth information when access is restricted
#[derive(Clone, Debug)]
pub struct AccessInfo {
    addr: SocketAddr,
    access_token: AccessToken,
    jwt_secret: JwtSecret,
}

impl AccessInfo {
    pub(crate) fn new(
        addr: SocketAddr,
        access_token: AccessToken,
        jwt_secret: Vec<u8>,
    ) -> Result<Self, crate::Error> {
        if let AccessToken::PreSharedToken(ref _token) = access_token {
            let jwt_secret = JwtSecret(jwt_secret);
            let access_token = access_token.clone();
            Ok(Self {
                addr,
                access_token,
                jwt_secret,
            })
        } else {
            Err(crate::Error::NonLocalhostRequiresPreSharedToken)
        }
    }
}

/// Access control method for the HTTP API
#[derive(Clone, Debug)]
pub enum AccessControl {
    /// Access is not restricted (for use with local IP addresses)
    Insecure(SocketAddr),
    /// Access is restricted
    WithToken(AccessInfo),
}

impl AccessControl {
    /// The address to bind the server to (e.g. `0.0.0.0`)
    pub(crate) fn bind_addr(&self) -> &SocketAddr {
        match self {
            AccessControl::Insecure(ref addr) => addr,
            AccessControl::WithToken(ref info) => &info.addr,
        }
    }

    pub(crate) fn token(&self) -> AccessToken {
        match self {
            AccessControl::Insecure(_) => AccessToken::NoToken,
            AccessControl::WithToken(ref info) => info.access_token.clone(),
        }
    }

    pub(crate) fn jwt_secret(&self) -> &[u8] {
        match self {
            AccessControl::Insecure(ref _addr) => b"insecure",
            AccessControl::WithToken(ref info) => &info.jwt_secret.0,
        }
    }
}
