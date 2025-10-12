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
    #[serde(rename = "width")]
    fixed_width: i32,
    expands: bool,
    resizable: bool,
    visible: bool,
    #[serde(rename = "type")]
    prop_type: Option<String>,
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
            prop_type: None,
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
            prop_type: Some(data.prop_type()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MyConfig {
    column: Vec<UnitColumn>,
}

pub fn save_column_config(data: &[UnitPropertySelection]) {
    let data_list: Vec<UnitColumn> = data.iter().map(UnitColumn::from).collect();

    let config = MyConfig { column: data_list };

    let xdg_config_home = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.config", home)
    });

    let parent_dir = Path::new(&xdg_config_home).join("sysd-manager");

    if let Err(e) = fs::create_dir_all(&parent_dir) {
        error!("Failed to create config directory {:?}: {}", parent_dir, e);
        return;
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
                ..Default::default()
            },
            UnitColumn {
                id: Some("beta".to_string()),
                title: Some("Beta Title".to_string()),
                fixed_width: 2,
                expands: false,
                resizable: true,
                visible: false,
                ..Default::default()
            },
            UnitColumn {
                id: Some("gamma".to_string()),
                title: Some("Gamma Title".to_string()),
                fixed_width: 3,
                expands: true,
                resizable: true,
                visible: true,
                prop_type: Some("i".to_string()),
            },
            UnitColumn {
                id: None,
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
        assert_eq!(config.column[0].id.as_deref(), Some("alpha"));
        assert_eq!(config.column[1].fixed_width, 2);
        assert!(config.column[2].visible);
        assert_eq!(config.column[3].title, None);
        assert_eq!(config.column[3].fixed_width, -1);
        assert_eq!(config.column[3].prop_type, Some("i".to_string()));
    }
}
