use std::fmt;

use secrecy::{ExposeSecret, Secret};
use webrtc::ice_transport::ice_server::RTCIceServer;

pub struct RtcConfig {
    pub address: String,
    pub port: u16,
    pub user: String,
    pub password: Secret<Option<String>>,
    pub ice_servers: Vec<RTCIceServer>,
}

impl fmt::Debug for RtcConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RtcConfig")
            .field("address", &self.address)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("ice_servers", &self.ice_servers)
            .finish()
    }
}

impl Default for RtcConfig {
    fn default() -> Self {
        let ice_servers = vec![RTCIceServer {
            urls: vec!["stun:stun.stunprotocol.org:3478".to_owned()],
            ..Default::default()
        }];
        Self {
            address: "127.0.0.1".to_string(),
            port: 0,
            user: Default::default(),
            password: Secret::new(None),
            ice_servers,
        }
    }
}

impl RtcConfig {
    pub fn set_password<S: AsRef<str>>(&mut self, password: S) {
        self.password = Secret::new(Some(password.as_ref().to_string()));
    }

    pub fn take_password(&mut self) -> Option<String> {
        let password = self.password.expose_secret().clone();
        self.password = Secret::new(None);
        password
    }

    pub fn base_url(&self) -> String {
        format!("ws://{}:{}/ws", self.address, self.port)
    }

    pub fn login_url(&self) -> String {
        format!("ws://{}:{}/ws/login", self.address, self.port)
    }
}

pub struct RtcConfigBuilder {
    pub address: String,
    pub port: u16,
    pub user: String,
    pub password: Secret<Option<String>>,
    pub ice_servers: Vec<RTCIceServer>,
}

impl Default for RtcConfigBuilder {
    fn default() -> Self {
        let ice_servers = vec![RTCIceServer {
            urls: vec!["stun:stun.stunprotocol.org:3478".to_owned()],
            ..Default::default()
        }];
        Self {
            address: "127.0.0.1".to_string(),
            port: 0,
            user: Default::default(),
            password: Secret::new(None),
            ice_servers,
        }
    }
}

impl RtcConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> RtcConfig {
        RtcConfig {
            address: self.address,
            port: self.port,
            user: self.user,
            password: self.password,
            ice_servers: self.ice_servers,
        }
    }

    pub fn address<S: AsRef<str>>(mut self, address: S) -> Self {
        self.address = address.as_ref().to_string();
        self
    }

    pub fn user<S: AsRef<str>>(mut self, user: S) -> Self {
        self.user = user.as_ref().to_string();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn password<S: AsRef<str>>(mut self, password: S) -> Self {
        self.password = Secret::new(Some(password.as_ref().to_string()));
        self
    }

    pub fn ice_servers(mut self, ice_servers: Vec<RTCIceServer>) -> Self {
        self.ice_servers = ice_servers;
        self
    }
}
