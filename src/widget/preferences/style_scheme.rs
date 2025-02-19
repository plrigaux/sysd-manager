use std::{collections::BTreeMap, sync::OnceLock};

use gtk::glib::GString;

#[derive(Debug, Default)]
pub struct StyleSchemes {
    pub name: String,
    pub dark: Option<String>,
    pub light: Option<String>,
}

impl StyleSchemes {
    fn create_style(style_str: GString) -> Self {
        let mut style = StyleSchemes::default();

        for (idx, part) in style_str.split('-').enumerate() {
            if idx == 0 {
                style.name = part.to_owned()
            } else {
                match part {
                    "dark" => style.dark = Some(style_str.clone().into()),
                    "light" => style.light = Some(style_str.clone().into()),
                    _ => {}
                }
            }
        }

        style
    }

    pub fn get_style_scheme_id(&self, is_dark: bool) -> String {
        if is_dark {
            self.get_dark()
        } else {
            self.get_light()
        }
    }

    pub fn get_dark(&self) -> String {
        match &self.dark {
            Some(s) => s.clone(),
            None => self.name.clone(),
        }
    }

    pub fn get_light(&self) -> String {
        match &self.light {
            Some(s) => s.clone(),
            None => self.name.clone(),
        }
    }
}

pub fn style_schemes() -> &'static BTreeMap<String, StyleSchemes> {
    static STYLE_SCHEMES: OnceLock<BTreeMap<String, StyleSchemes>> = OnceLock::new();
    STYLE_SCHEMES.get_or_init(|| {
        let mut map: BTreeMap<String, StyleSchemes> = BTreeMap::new();

        for scheme in sourceview5::StyleSchemeManager::new().scheme_ids() {
            println!("{}", scheme);
            let style = StyleSchemes::create_style(scheme);
            let key = style.name.clone();
            if let Some(stored_style) = map.get_mut(&key) {
                if stored_style.dark.is_none() && style.dark.is_some() {
                    stored_style.dark = style.dark
                } else if stored_style.light.is_none() && style.light.is_some() {
                    stored_style.light = style.light
                }
            } else {
                map.insert(style.name.clone(), style);
            }
        }
        map
    })
}
