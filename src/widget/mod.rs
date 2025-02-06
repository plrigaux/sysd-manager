use gtk::pango::FontDescription;

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
    SetVisibleOnPage(bool),
}
