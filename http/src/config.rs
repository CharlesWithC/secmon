use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

const DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_PORT: u16 = 9993;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub http: HttpConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpConfig {
    pub bind_ip: IpAddr,
    pub bind_port: u16,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            bind_ip: DEFAULT_IP,
            bind_port: DEFAULT_PORT,
        }
    }
}
