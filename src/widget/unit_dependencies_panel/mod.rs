mod imp;

use std::collections::HashSet;

use gtk::{glib, subclass::prelude::*};

use crate::systemd::enums::DependencyType;

use super::{app_window::AppWindow, InterPanelAction};

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

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        self.imp().set_inter_action(action);
    }
}

impl Default for UnitDependenciesPanel {
    fn default() -> Self {
        UnitDependenciesPanel::new()
    }
}
