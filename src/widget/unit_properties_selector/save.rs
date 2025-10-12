use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{env, fs};

use crate::widget::unit_properties_selector::data_selection::UnitPropertySelection;

const UNIT_COLUMNS: &str = "unit_columns.toml";

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct UnitColumn {
    pub id: String,
    pub title: Option<String>,
    #[serde(rename = "width")]
    pub fixed_width: i32,
    pub expands: bool,
    pub resizable: bool,
    pub visible: bool,
    #[serde(rename = "type")]
    pub prop_type: Option<String>,
}

impl Default for UnitColumn {
    fn default() -> Self {
        Self {
            id: "".to_owned(),
            title: None,
            fixed_width: -1,
            expands: false,
            resizable: false,
            visible: true,
            prop_type: None,
        }
    }
}

impl UnitColumn {
    pub fn from(data: &UnitPropertySelection) -> Self {
        Self {
            id: data.id().map(|s| s.to_string()).unwrap_or_default(),
            title: data.title().map(|s| s.to_string()),
            fixed_width: data.fixed_width(),
            expands: data.expands(),
            resizable: data.resizable(),
            visible: data.visible(),
            prop_type: data.prop_type(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MyConfig {
    pub column: Vec<UnitColumn>,
}

pub fn save_column_config(data: &[UnitPropertySelection]) {
    let data_list: Vec<UnitColumn> = data.iter().map(UnitColumn::from).collect();

    let config = MyConfig { column: data_list };

    let sysd_manager_config_dir = get_sysd_manager_config_dir();

    if let Err(e) = fs::create_dir_all(&sysd_manager_config_dir) {
        error!(
            "Failed to create config directory {:?}: {}",
            sysd_manager_config_dir, e
        );
        return;
    }

    let config_path = sysd_manager_config_dir.join(UNIT_COLUMNS);

    if let Err(e) = save_to_toml_file(&config, &config_path) {
        error!(
            "Failed to save column config to TOML file: {:?} {:?}",
            config_path, e
        );
    } else {
        info!("Column config saved to {:?}", config_path);
    }
}

fn get_sysd_manager_config_dir() -> std::path::PathBuf {
    let xdg_config_home = get_xdg_config_home();

    Path::new(&xdg_config_home).join("sysd-manager")
}

fn get_xdg_config_home() -> String {
    env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.config", home)
    })
}

pub fn save_to_toml_file(data: &MyConfig, path: &Path) -> std::io::Result<()> {
    let toml_str = toml::to_string_pretty(data).expect("Failed to serialize data to TOML");
    let mut file = File::create(path)?;
    file.write_all(toml_str.as_bytes())?;
    Ok(())
}

pub fn load_column_config() -> Option<MyConfig> {
    let sysd_manager_config_dir = get_sysd_manager_config_dir();

    if !sysd_manager_config_dir.exists() {
        info!(
            "Config directory {:?} does not exist. Using default configuration.",
            sysd_manager_config_dir
        );
        return None;
    }

    let config_path = sysd_manager_config_dir.join(UNIT_COLUMNS);

    if !config_path.exists() {
        info!(
            "Config file {:?} does not exist. Using default configuration.",
            config_path
        );
        return None;
    }

    match fs::read_to_string(&config_path) {
        Ok(toml_str) => match toml::from_str::<MyConfig>(&toml_str) {
            Ok(config) => Some(config),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_save_array_of_structs_to_toml() {
        let data_list = vec![
            UnitColumn {
                id: "alpha".to_string(),
                title: Some("Alpha Title".to_string()),
                fixed_width: 1,
                expands: true,
                resizable: false,
                visible: true,
                ..Default::default()
            },
            UnitColumn {
                id: "beta".to_string(),
                title: Some("Beta Title".to_string()),
                fixed_width: 2,
                expands: false,
                resizable: true,
                visible: false,
                ..Default::default()
            },
            UnitColumn {
                id: "gamma".to_string(),
                title: Some("Gamma Title".to_string()),
                fixed_width: 3,
                expands: true,
                resizable: true,
                visible: true,
                prop_type: Some("i".to_string()),
            },
            UnitColumn {
                id: "".to_string(),
                title: None,
                fixed_width: 3,
                expands: true,
                resizable: true,
                visible: true,
                ..Default::default()
            },
        ];

        let config = MyConfig { column: data_list };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize array to TOML");

        println!("{}", toml_str);

        // Check that each struct appears as a TOML table
        assert!(toml_str.contains("id = \"alpha\""));
        assert!(toml_str.contains("id = \"beta\""));
        assert!(toml_str.contains("title = \"Gamma Title\""));
        assert!(toml_str.matches("[").count() >= 4); // At least 4 tables
    }

    #[test]
    fn test_load_multiple_structs_from_toml_file() {
        let toml_content = r#"
            [[column]]
            id = "alpha"
            title = "Alpha Title"
            fixed_width = 1
            expands = true
            resizable = false
            visible = true

            [[column]]
            id = "beta"
            title = "Beta Title"
            width = 2
            expands = false
            resizable = true
            visible = false

            [[column]]
            id = "gamma"
            title = "Gamma Title"
            width = 3
            expands = true
            resizable = true
            visible = true

            [[column]]         
            expands = true
            resizable = true
            visible = true
            type = "i"
        "#;

        let config: MyConfig = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert!(config.column.len() >= 4);
        assert_eq!(config.column[0].id.as_str(), "alpha");
        assert_eq!(config.column[1].fixed_width, 2);
        assert!(config.column[2].visible);
        assert_eq!(config.column[3].title, None);
        assert_eq!(config.column[3].fixed_width, -1);
        assert_eq!(config.column[3].prop_type, Some("i".to_string()));
    }
}
