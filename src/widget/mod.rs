use gtk::{
    ffi::GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
    pango::{self, FontDescription},
    prelude::WidgetExt,
};
use log::debug;

pub mod app_window;
pub mod button_icon;
pub mod grid_cell;
pub mod info_window;
pub mod journal;
pub mod kill_panel;
pub mod menu_button;
pub mod preferences;
pub mod unit_control_panel;
pub mod unit_dependencies_panel;
pub mod unit_file_panel;
pub mod unit_info;
pub mod unit_list;

pub enum InterPanelAction<'a> {
    SetFont(&'a FontDescription),
    SetDark(bool),
}

pub fn set_text_view_font(font_description: &FontDescription, text_view: &gtk::TextView) {
    let family = font_description.family();
    let size = font_description.size() / pango::SCALE;

    debug!("set font {:?}", font_description.to_string());
    debug!(
        "set familly {:?} gravity {:?} weight {:?} size {} variations {:?} stretch {:?}",
        font_description.family(),
        font_description.gravity(),
        font_description.weight(),
        font_description.size(),
        font_description.variations(),
        font_description.stretch(),
    );
    // let pango_context = self.unit_info_textview.pango_context();

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
    css.push_str("}");

    provider.load_from_string(&css);

    gtk::style_context_add_provider_for_display(
        &text_view.display(),
        &provider,
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION as u32,
    );
}
