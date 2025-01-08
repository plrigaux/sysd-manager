mod imp;

use gtk::{glib, subclass::prelude::*};

use crate::systemd::enums::DependencyType;

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

    pub(super) fn replace_dependency_type(&self, dt: DependencyType) -> DependencyType  {
        self.imp().dependency_type.replace(dt)
    }

    pub(super) fn update_dependencies(&self) {
        self.imp().update_dependencies()
    }
}
