mod imp;

use std::collections::HashSet;

use gtk::FilterChange;
use log::debug;

use crate::gtk::prelude::FilterExt;
use crate::gtk::{glib, subclass::prelude::*};

use super::unit_dependencies_panel::UnitDependenciesPanel;

glib::wrapper! {
    pub struct ExMenuButton(ObjectSubclass<imp::ExMenuButtonImpl>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl ExMenuButton {
    pub fn new(label: &str) -> Self {
        let obj: ExMenuButton = glib::Object::new();
        obj.set_label(label);

        obj
    }

    pub fn add_item(&mut self, label: &str) {
        self.imp().add_item(label);
    }

    pub fn contains_value(&self, value: Option<&str>) -> bool {
        let imp = self.imp();
        imp.contains_value(value)
    }

    pub fn set_on_close(&self, closure: OnClose) {
        self.imp().set_on_close(closure)
    }
}

impl Default for ExMenuButton {
    fn default() -> Self {
        ExMenuButton::new("")
    }
}

#[derive(Debug, Default)]
pub struct OnClose {
    filter: Option<gtk::CustomFilter>,
    dependencies: Option<UnitDependenciesPanel>,
}

impl OnClose {
    pub fn new_filter(filter: &gtk::CustomFilter) -> Self {
        OnClose {
            filter: Some(filter.clone()),
            dependencies: None,
        }
    }

    pub fn new_dep(dep: &UnitDependenciesPanel) -> Self {
        OnClose {
            filter: None,
            dependencies: Some(dep.clone()),
        }
    }

    pub fn old_new_compare(&self, old: &HashSet<String>, new: &HashSet<String>) {
        if let Some(filter) = &self.filter {
            let filter_change = Self::determine_filter_change(new, old);

            if let Some(filter_change) = filter_change {
                filter.changed(filter_change);
                debug!("Filter change Level {filter_change:?}");
            }
        } else if let Some(dependencies) = &self.dependencies
            && !old.eq(new)
        {
            dependencies.update_dependencies_filtered(new);
        }
    }

    fn determine_filter_change(
        new_set: &HashSet<String>,
        old_set: &HashSet<String>,
    ) -> Option<FilterChange> {
        if old_set.is_empty() && !new_set.is_empty() {
            Some(FilterChange::MoreStrict)
        } else if !old_set.is_empty() && new_set.is_empty() {
            Some(FilterChange::LessStrict)
        } else if old_set.len() == new_set.len() {
            if old_set.iter().all(|item| new_set.contains(item)) {
                None
            } else {
                Some(FilterChange::Different)
            }
        } else if old_set.len() > new_set.len() {
            if new_set.iter().all(|item| old_set.contains(item)) {
                Some(FilterChange::MoreStrict)
            } else {
                Some(FilterChange::Different)
            }
        } else if old_set.iter().all(|item| new_set.contains(item)) {
            Some(FilterChange::LessStrict)
        } else {
            Some(FilterChange::Different)
        }
    }
}
