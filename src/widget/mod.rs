use gtk::{
    ffi::GTK_STYLE_PROVIDER_PRIORITY_APPLICATION, pango::FontDescription, prelude::WidgetExt,
};
use log::info;

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
    SetFont(Option<&'a FontDescription>),
    SetFontProvider(Option<&'a gtk::CssProvider>, Option<&'a gtk::CssProvider>),
    SetDark(bool),
}

pub fn set_text_view_font(
    old_provider: Option<&gtk::CssProvider>,
    new_provider: Option<&gtk::CssProvider>,
    text_view: &gtk::TextView,
) {
    if let Some(old_provider) = old_provider {
        info!("set font default");
        let provider = gtk::CssProvider::new();
        let css = String::from("textview {}");
        provider.load_from_string(&css);

        gtk::style_context_remove_provider_for_display(&text_view.display(), old_provider);
    };

    if let Some(new_provider) = new_provider {
        gtk::style_context_add_provider_for_display(
            &text_view.display(),
            new_provider,
            GTK_STYLE_PROVIDER_PRIORITY_APPLICATION as u32,
        );
    }
}
