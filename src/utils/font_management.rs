use crate::widget::preferences::data::PREFERENCES;
use gtk::{
    ffi::GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
    gdk,
    pango::{self, FontDescription},
    prelude::WidgetExt,
};
use std::sync::{LazyLock, OnceLock, RwLock};
use tracing::{debug, info, warn};

static FONT_CONTEXT: LazyLock<FontContext> = LazyLock::new(FontContext::default);

static FONT_DEFAULT: OnceLock<FontDescription> = OnceLock::new();

#[derive(Default, Debug)]
pub struct FontContext {
    font_description: RwLock<FontDescription>,
    font_label: RwLock<String>,
}

impl FontContext {
    pub fn set_font_description(&self, font_description: FontDescription) {
        let mut font_label_w = self.font_label.write().expect("supposed to write");
        *font_label_w = font_description.to_string();
        let mut font_description_w = self.font_description.write().expect("supposed to write");
        *font_description_w = font_description;
    }

    pub fn set_default_font_description(&self, font_description: FontDescription) {
        let mut font_label_w = self.font_label.write().expect("supposed to write");
        *font_label_w = format!("Default - {}", font_description);
        let mut font_description_w = self.font_description.write().expect("supposed to write");
        *font_description_w = font_description;
    }

    pub fn font_description(&self) -> FontDescription {
        let font_description = self.font_description.read().expect("supposed to read");
        font_description.clone()
    }

    pub fn font_label(&self) -> String {
        let font_description = self.font_label.read().expect("supposed to read");
        font_description.clone()
    }
}

pub fn font_description() -> FontDescription {
    FONT_CONTEXT.font_description()
}

pub fn set_font_description(font_description: FontDescription) {
    info!("New Font {:?}", font_description.to_string());
    PREFERENCES.set_font(&font_description);
    FONT_CONTEXT.set_font_description(font_description);
}

pub fn set_default_font_description() {
    info!("Default Font");
    PREFERENCES.set_font_default();
    if let Some(fd) = FONT_DEFAULT.get() {
        FONT_CONTEXT.set_font_description(fd.clone());
    }
}

pub fn font_label() -> String {
    FONT_CONTEXT.font_label()
}

pub fn is_default_font(family: &str, size: u32) -> bool {
    family.is_empty() && size == 0
}

pub fn has_custom_font() -> Option<FontDescription> {
    let family = PREFERENCES.font_family();
    let size = PREFERENCES.font_size();

    if !is_default_font(&family, size) {
        let mut font_description = FontDescription::new();

        if !family.is_empty() {
            font_description.set_family(&family);
        }

        if size != 0 {
            let scaled_size = size as i32 * pango::SCALE;
            font_description.set_size(scaled_size);
        }

        println!("ffffffffff {:?} {}", family, size);
        FONT_CONTEXT.set_font_description(font_description.clone());
        Some(font_description)
    } else {
        None
    }
}

pub fn create_provider(font_description: &Option<&FontDescription>) -> Option<gtk::CssProvider> {
    let Some(font_description) = font_description else {
        info!("set font default");

        //gtk::style_context_remove_provider_for_display(&text_view.display(), &provider);
        return None;
    };

    let family = font_description.family();
    let size = font_description.size() / pango::SCALE;

    info!("set font {:?}", font_description.to_string());
    debug!(
        "set familly {:?} gravity {:?} weight {:?} size {} variations {:?} stretch {:?}",
        font_description.family(),
        font_description.gravity(),
        font_description.weight(),
        font_description.size(),
        font_description.variations(),
        font_description.stretch(),
    );

    let provider = gtk::CssProvider::new();

    let mut css = String::with_capacity(100);

    css.push_str("textview {");
    css.push_str("font-size: ");
    css.push_str(&size.to_string());
    css.push_str("px;\n");

    if let Some(family) = family {
        css.push_str("font-family: ");
        css.push('"');
        css.push_str(family.as_str());
        css.push_str("\";\n");
    }
    css.push('}');

    provider.load_from_string(&css);

    Some(provider)
}

pub fn set_text_view_font(
    old_provider: Option<&gtk::CssProvider>,
    new_provider: Option<&gtk::CssProvider>,
    text_view: &gtk::TextView,
) {
    set_text_view_font_display(old_provider, new_provider, &text_view.display())
}

pub fn set_text_view_font_display(
    old_provider: Option<&gtk::CssProvider>,
    new_provider: Option<&gtk::CssProvider>,
    display: &gdk::Display,
) {
    if let Some(old_provider) = old_provider {
        info!("rem old font provider");
        let provider = gtk::CssProvider::new();
        let css = String::from("textview {}");
        provider.load_from_string(&css);

        gtk::style_context_remove_provider_for_display(display, old_provider);
    };

    if let Some(new_provider) = new_provider {
        info!("add new font provider");
        gtk::style_context_add_provider_for_display(
            display,
            new_provider,
            GTK_STYLE_PROVIDER_PRIORITY_APPLICATION as u32,
        );
    }
}

pub fn set_font_default_context(text_view: &gtk::TextView) {
    let context = text_view.pango_context();
    let font_description = context.font_description();
    if let Some(font_description) = font_description {
        info!("Default Font Description {font_description}");
        if let Err(err) = FONT_DEFAULT.set(font_description.clone()) {
            warn!("Font aready set {err}");
        };
        FONT_CONTEXT.set_default_font_description(font_description);
    } else {
        warn!("NO FONT Description")
    }
}
