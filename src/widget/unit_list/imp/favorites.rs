use crate::widget::{
    unit_list::imp::UnitKey,
    unit_properties_selector::save::{get_sysd_manager_config_dir, save_to_toml_file},
};
use base::enums::UnitDBusLevel;
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{error, info, warn};

const FAVORITES: &str = "favorites.toml";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Favorites {
    #[serde(rename = "column")]
    pub favs: Vec<Favorite>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct Favorite {
    pub bus: String,
    pub unit: String,
}

impl Favorite {
    fn from(key: &UnitKey) -> Self {
        Self {
            bus: key.level.as_str().to_owned(),
            unit: key.primary.to_owned(),
        }
    }
}

pub(super) fn save_favorites(favorite: &[UnitKey]) {
    let c: Vec<Favorite> = favorite.iter().map(Favorite::from).collect();

    let config = Favorites { favs: c };

    let sysd_manager_config_dir = get_sysd_manager_config_dir();

    if let Err(e) = fs::create_dir_all(&sysd_manager_config_dir) {
        error!(
            "Failed to create config directory {:?}: {}",
            sysd_manager_config_dir, e
        );
        return;
    }

    let config_path = sysd_manager_config_dir.join(FAVORITES);

    if let Err(e) = save_to_toml_file(&config, &config_path) {
        error!(
            "Failed to save column config to TOML file: {:?} {:?}",
            config_path, e
        );
    } else {
        info!("Column config saved to {:?}", config_path);
    }
}

pub(super) fn load_favorites() -> Option<Favorites> {
    let sysd_manager_config_dir = get_sysd_manager_config_dir();

    if !sysd_manager_config_dir.exists() {
        info!(
            "Config directory {:?} does not exist. Using default configuration.",
            sysd_manager_config_dir
        );
        return None;
    }

    let config_path = sysd_manager_config_dir.join(FAVORITES);

    if !config_path.exists() {
        info!(
            "Config file {:?} does not exist. Using default configuration.",
            config_path
        );
        return None;
    }

    match fs::read_to_string(&config_path) {
        Ok(toml_str) => match toml::from_str::<Favorites>(&toml_str) {
            Ok(config) => {
                if config.favs.is_empty() {
                    warn!("Loaded config is empty, FALLBACK on default");
                    None
                } else {
                    Some(config)
                }
            }
            Err(e) => {
                error!("Failed to parse TOML from {:?}: {}", config_path, e);
                None
            }
        },
        Err(e) => {
            error!("Failed to read config file {:?}: {}", config_path, e);
            None
        }
    }
}
