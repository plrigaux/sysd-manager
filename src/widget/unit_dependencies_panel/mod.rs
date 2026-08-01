mod imp;

use super::{InterPanelMessage, app_window::AppWindow};
use crate::{systemd::enums::DependencyType, widget::text_search::TextSearchEntry};
use gtk::{glib, subclass::prelude::*};
use std::collections::HashSet;

glib::wrapper! {
    pub struct UnitDependenciesPanel(ObjectSubclass<imp::UnitDependenciesPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitDependenciesPanel {
    pub fn new() -> Self {
        let obj: UnitDependenciesPanel = glib::Object::new();
        obj
    }

    pub(super) fn replace_dependency_type(&self, dt: DependencyType) -> DependencyType {
        self.imp().dependency_type.replace(dt)
    }

    pub(super) fn update_dependencies(&self) {
        self.imp().update_dependencies()
    }

    pub(super) fn update_dependencies_filtered(&self, unit_type_filter: &HashSet<String>) {
        self.imp().update_dependencies_filtered(unit_type_filter)
    }

    pub fn register(&self, app_window: &AppWindow) {
        self.imp().register(app_window);
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        self.imp().set_inter_message(action);
    }

    pub fn focus_text_search(&self) {
        self.imp().focus_text_search()
    }

    pub fn set_text_search_entry(&self, text_search_entry: &TextSearchEntry) {
        self.imp().set_text_search_entry(text_search_entry)
    }
}

impl Default for UnitDependenciesPanel {
    fn default() -> Self {
        UnitDependenciesPanel::new()
    }
}
