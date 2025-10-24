use std::rc::Rc;

use gtk::pango::FontDescription;

use crate::systemd::{BootFilter, data::UnitInfo};

pub mod app_window;
pub mod clean_dialog;
pub mod control_action_dialog;
pub mod grid_cell;
pub mod info_window;
pub mod journal;
pub mod kill_panel;
pub mod menu_button;
pub mod preferences;
pub mod signals_dialog;
pub mod unit_control_panel;
pub mod unit_dependencies_panel;
pub mod unit_file_panel;
pub mod unit_info;
pub mod unit_list;
pub mod unit_properties_selector;

pub enum InterPanelMessage<'a> {
    Font(Option<&'a FontDescription>),
    FontProvider(Option<&'a gtk::CssProvider>, Option<&'a gtk::CssProvider>),
    IsDark(bool),
    PanelVisible(bool),
    NewStyleScheme(Option<&'a str>),
    FileLineNumber(bool),
    UnitChange(Option<&'a UnitInfo>),
    JournalFilterBoot(BootFilter),
    StartUnit(gtk::Button, UnitInfo, Rc<Box<dyn Fn()>>),
    StopUnit(gtk::Button, UnitInfo, Rc<Box<dyn Fn()>>),
    ReStartUnit(gtk::Button, UnitInfo, Rc<Box<dyn Fn()>>),
    EnableUnit(UnitInfo, Rc<Box<dyn Fn()>>),
    DisableUnit(UnitInfo, Rc<Box<dyn Fn()>>),
    ReenableUnit(UnitInfo, Rc<Box<dyn Fn()>>),
    MaskUnit(&'a gtk::Button, &'a UnitInfo),
    UnMaskUnit(&'a gtk::Button, &'a UnitInfo),
    ReloadUnit(gtk::Button, UnitInfo, Rc<Box<dyn Fn()>>),
}
