use std::{collections::BTreeMap, sync::OnceLock};

use gtk::glib::GString;
use sourceview5::prelude::BufferExt;
use tracing::{debug, info, warn};

use crate::{consts::ADWAITA, systemd_gui::is_dark};

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

    fn get_dark(&self) -> String {
        match &self.dark {
            Some(s) => s.clone(),
            None => self.name.clone(),
        }
    }

    fn get_light(&self) -> String {
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
            debug!("style scheme: {scheme}");
            let style = StyleSchemes::create_style(scheme);

            if let Some(stored_style) = map.get_mut(&style.name) {
                if stored_style.dark.is_none() && style.dark.is_some() {
                    stored_style.dark = style.dark
                } else if stored_style.light.is_none() && style.light.is_some() {
                    stored_style.light = style.light
                }
            } else {
                let key = style.name.clone();
                map.insert(key, style);
            }
        }
        map
    })
}

pub fn set_new_style_scheme(buffer: &sourceview5::Buffer, style_scheme_id: Option<&str>) {
    info!("Set new style scheme {style_scheme_id:?}");

    match style_scheme_id {
        Some("") | None => {
            buffer.set_style_scheme(None);
        }
        Some(style_scheme_id) => {
            let style_schemes_map: &'static std::collections::BTreeMap<
                String,
                crate::widget::preferences::style_scheme::StyleSchemes,
            > = style_schemes();

            debug!("{style_schemes_map:#?}");
            /*             if style_scheme_id.is_empty() {
                style_scheme_id = ADWAITA;
            } */

            let style_scheme_st = style_schemes_map.get(style_scheme_id);

            let style_sheme_st = match style_scheme_st {
                Some(ss) => ss,
                None => {
                    info!(
                        "Style scheme id \"{style_scheme_id}\" not found in {:?}",
                        style_schemes_map.keys().collect::<Vec<_>>()
                    );

                    //fallback on style Adwaita
                    if let Some(style_scheme_st) = style_schemes_map.get(ADWAITA) {
                        style_scheme_st
                    } else
                    //fallback on first item
                    if let Some((_, style_scheme_st)) = style_schemes_map.first_key_value() {
                        style_scheme_st
                    } else {
                        return;
                    }
                }
            };

            let scheme_id = &style_sheme_st.get_style_scheme_id(is_dark());

            if let Some(ref scheme) = sourceview5::StyleSchemeManager::new().scheme(scheme_id) {
                info!("Style Scheme found for id {scheme_id:?}");
                buffer.set_style_scheme(Some(scheme));
            } else {
                warn!("No Style Scheme found for id {scheme_id:?}")
            }
        }
    }
}
