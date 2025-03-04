use gtk::pango::FontDescription;

pub mod app_window;
pub mod button_icon;
pub mod clean_dialog;
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
    Font(Option<&'a FontDescription>),
    FontProvider(Option<&'a gtk::CssProvider>, Option<&'a gtk::CssProvider>),
    IsDark(bool),
    PanelVisible(bool),
    NewStyleScheme(Option<&'a str>),
    FileLineNumber(bool),
}
