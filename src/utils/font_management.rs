use std::sync::{LazyLock, RwLock};

use gtk::{
    ffi::GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
    gdk,
    pango::{self, FontDescription},
    prelude::WidgetExt,
};
use log::{debug, info, warn};

pub static FONT_CONTEXT: LazyLock<FontContext> = LazyLock::new(FontContext::default);

#[derive(Default, Debug)]
pub struct FontContext {
    //font_family: RwLock<String>,
    //font_size: RwLock<i32>,
    font_description: RwLock<FontDescription>,
}

impl FontContext {
    pub fn set_font_description(&self, font_description: FontDescription) {
        let mut font_description_w = self.font_description.write().expect("supposed to write");
        *font_description_w = font_description;
    }

    pub fn font_description(&self) -> FontDescription {
        let font_description = self.font_description.read().expect("supposed to read");
        font_description.clone()
    }
}

pub fn is_default_font(family: &str, size: u32) -> bool {
    family.is_empty() && size == 0
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

pub fn set_font_context(text_view: &gtk::TextView) {
    let context = text_view.pango_context();
    let font_description = context.font_description();
    if let Some(font_description) = font_description {
        info!("Font description {font_description}");
        FONT_CONTEXT.set_font_description(font_description);
    } else {
        warn!("NO FONT Description")
    }
}
