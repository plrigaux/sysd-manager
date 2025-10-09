use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{env, fs};

use crate::widget::unit_properties_selector::data_selection::UnitPropertySelection;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
struct UnitColumn {
    id: Option<String>,
    title: Option<String>,
    fixed_width: i32,
    expands: bool,
    resizable: bool,
    visible: bool,
}

impl Default for UnitColumn {
    fn default() -> Self {
        Self {
            id: None,
            title: None,
            fixed_width: -1,
            expands: false,
            resizable: false,
            visible: true,
        }
    }
}

impl UnitColumn {
    pub fn from(data: &UnitPropertySelection) -> Self {
        Self {
            id: data.id().map(|s| s.to_string()),
            title: data.title().map(|s| s.to_string()),
            fixed_width: data.fixed_width(),
            expands: data.expands(),
            resizable: data.resizable(),
            visible: data.visible(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MyConfig {
    data_items: Vec<UnitColumn>,
}

pub fn save_column_config(data: &[UnitPropertySelection]) {
    let data_list: Vec<UnitColumn> = data.iter().map(UnitColumn::from).collect();

    let config = MyConfig {
        data_items: data_list,
    };

    let xdg_config_home = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.config", home)
    });

    let parent_dir = Path::new(&xdg_config_home).join("sysd-manager");

    if let Err(e) = fs::create_dir_all(&parent_dir) {
        error!("Failed to create config directory {:?}: {}", parent_dir, e);
    }

    let config_path = parent_dir.join("unit_columns.toml");

    if let Err(e) = save_to_toml_file(&config, &config_path) {
        error!(
            "Failed to save column config to TOML file: {:?} {:?}",
            config_path, e
        );
    } else {
        info!("Column config saved to {:?}", config_path);
    }
}

pub fn save_to_toml_file(data: &MyConfig, path: &Path) -> std::io::Result<()> {
    let toml_str = toml::to_string_pretty(data).expect("Failed to serialize data to TOML");
    let mut file = File::create(path)?;
    file.write_all(toml_str.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_save_array_of_structs_to_toml() {
        let data_list = vec![
            UnitColumn {
                id: Some("alpha".to_string()),
                title: Some("Alpha Title".to_string()),
                fixed_width: 1,
                expands: true,
                resizable: false,
                visible: true,
            },
            UnitColumn {
                id: Some("beta".to_string()),
                title: Some("Beta Title".to_string()),
                fixed_width: 2,
                expands: false,
                resizable: true,
                visible: false,
            },
            UnitColumn {
                id: Some("gamma".to_string()),
                title: Some("Gamma Title".to_string()),
                fixed_width: 3,
                expands: true,
                resizable: true,
                visible: true,
            },
            UnitColumn {
                id: None,
                title: None,
                fixed_width: 3,
                expands: true,
                resizable: true,
                visible: true,
            },
        ];

        let config = MyConfig {
            data_items: data_list,
        };

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
            [[data_items]]
            id = "alpha"
            title = "Alpha Title"
            fixed_width = 1
            expands = true
            resizable = false
            visible = true

            [[data_items]]
            id = "beta"
            title = "Beta Title"
            fixed_width = 2
            expands = false
            resizable = true
            visible = false

            [[data_items]]
            id = "gamma"
            title = "Gamma Title"
            fixed_width = 3
            expands = true
            resizable = true
            visible = true

            [[data_items]]         
            expands = true
            resizable = true
            visible = true
        "#;

        let config: MyConfig = toml::from_str(toml_content).expect("Failed to parse TOML");

        assert!(config.data_items.len() >= 4);
        assert_eq!(config.data_items[0].id.as_deref(), Some("alpha"));
        assert_eq!(config.data_items[1].fixed_width, 2);
        assert!(config.data_items[2].visible);
        assert_eq!(config.data_items[3].title, None);
        assert_eq!(config.data_items[3].fixed_width, -1);
    }
}
