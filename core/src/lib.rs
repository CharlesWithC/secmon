pub mod models {
    pub mod hub;
    pub mod node;
    pub mod packet;
}
pub mod traits {
    pub mod exec;
    pub mod iosered;
    pub mod tryclone;
}
pub mod config;
pub mod crypto;
pub mod logging;
pub mod utils;

pub const GENERIC_OPTIONS: &str = "\
  -c, --config <file>               specify a config file, otherwise tries:
                                    $XDG_CONFIG_HOME/secmon/config.toml,
                                    /etc/secmon/config.toml in order by default
  -v, --verbose                     verbose mode";
