use std::collections::BTreeMap;

use ruma::ServerName;
use serde::{de::IgnoredAny, Deserialize};
use tracing::warn;

mod proxy;

use self::proxy::ProxyConfig;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub server_name: Box<ServerName>,
    #[serde(default = "default_database_backend")]
    pub database_backend: String,
    pub database_path: String,
    #[serde(default = "default_db_cache_capacity_mb")]
    pub db_cache_capacity_mb: f64,
    #[serde(default = "default_conduit_cache_capacity_modifier")]
    pub conduit_cache_capacity_modifier: f64,
    #[serde(default = "default_rocksdb_max_open_files")]
    pub rocksdb_max_open_files: i32,
    #[serde(default = "default_pdu_cache_capacity")]
    pub pdu_cache_capacity: u32,
    #[serde(default = "default_cleanup_second_interval")]
    pub cleanup_second_interval: u32,
    #[serde(default = "default_max_request_size")]
    pub max_request_size: u32,
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: u16,
    #[serde(default = "false_fn")]
    pub allow_registration: bool,
    #[serde(default = "true_fn")]
    pub allow_encryption: bool,
    #[serde(default = "false_fn")]
    pub allow_federation: bool,
    #[serde(default = "true_fn")]
    pub allow_room_creation: bool,
    #[serde(default = "false_fn")]
    pub allow_jaeger: bool,
    #[serde(default = "false_fn")]
    pub tracing_flame: bool,
    #[serde(default)]
    pub proxy: ProxyConfig,
    pub jwt_secret: Option<String>,
    #[serde(default = "Vec::new")]
    pub trusted_servers: Vec<Box<ServerName>>,
    #[serde(default = "default_log")]
    pub log: String,
    #[serde(default)]
    pub turn_username: String,
    #[serde(default)]
    pub turn_password: String,
    #[serde(default = "Vec::new")]
    pub turn_uris: Vec<String>,
    #[serde(default)]
    pub turn_secret: String,
    #[serde(default = "default_turn_ttl")]
    pub turn_ttl: u64,

    #[serde(flatten)]
    pub catchall: BTreeMap<String, IgnoredAny>,
}

const DEPRECATED_KEYS: &[&str] = &["cache_capacity"];

impl Config {
    pub fn warn_deprecated(&self) {
        let mut was_deprecated = false;
        for key in self
            .catchall
            .keys()
            .filter(|key| DEPRECATED_KEYS.iter().any(|s| s == key))
        {
            warn!("Config parameter {} is deprecated", key);
            was_deprecated = true;
        }

        if was_deprecated {
            warn!("Read conduit documentation and check your configuration if any new configuration parameters should be adjusted");
        }
    }
}

fn false_fn() -> bool {
    false
}

fn true_fn() -> bool {
    true
}

fn default_database_backend() -> String {
    "sqlite".to_owned()
}

fn default_db_cache_capacity_mb() -> f64 {
    10.0
}

fn default_conduit_cache_capacity_modifier() -> f64 {
    1.0
}

fn default_rocksdb_max_open_files() -> i32 {
    20
}

fn default_pdu_cache_capacity() -> u32 {
    150_000
}

fn default_cleanup_second_interval() -> u32 {
    1 * 60 // every minute
}

fn default_max_request_size() -> u32 {
    20 * 1024 * 1024 // Default to 20 MB
}

fn default_max_concurrent_requests() -> u16 {
    100
}

fn default_log() -> String {
    "info,state_res=warn,rocket=off,_=off,sled=off".to_owned()
}

fn default_turn_ttl() -> u64 {
    60 * 60 * 24
}
