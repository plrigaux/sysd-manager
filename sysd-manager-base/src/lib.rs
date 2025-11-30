use std::env;

use tracing::info;

pub const PROXY_SERVICE: &str = "sysd-manager-proxy";
pub const PROXY_SERVICE_DEV: &str = "sysd-manager-proxy-dev";

#[derive(Debug, PartialEq, Eq)]
pub enum RunMode {
    Normal,
    Development,
    Both,
}

impl RunMode {
    pub fn from_flags(dev: bool, normal: bool) -> Self {
        let cargo_in_use = env::var("CARGO");

        match (dev, normal, cargo_in_use) {
            (true, true, _) => RunMode::Both,
            (true, false, _) => RunMode::Development,
            (false, true, _) => RunMode::Normal,
            (false, false, Ok(_)) => {
                info!("The program is being run by cargo. --> Assume Development mode.");
                RunMode::Development
            }
            (false, false, Err(_)) => RunMode::Normal,
        }
    }
}
