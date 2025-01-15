mod imp;

use std::collections::HashSet;

use gtk::FilterChange;
use log::debug;

use crate::gtk::prelude::FilterExt;
use crate::gtk::{glib, subclass::prelude::*};

glib::wrapper! {
    pub struct ExMenuButton(ObjectSubclass<imp::ExMenuButtonImpl>)
        @extends gtk::Widget,
        @implements gtk::Buildable;
}

impl ExMenuButton {
    pub fn new(label: &str) -> Self {
        let obj: ExMenuButton = glib::Object::new();
        obj.set_label(label);

        obj
    }

    pub fn add_item(&mut self, label: &str) {
        let binding = self.imp();

        binding.add_item(label);
    }

    pub fn contains_value(&self, value: Option<&str>) -> bool {
        let imp = self.imp();
        imp.contains_value(value)
    }

    pub fn set_on_close(&self, closure: OnClose) {
        let imp: &imp::ExMenuButtonImpl = self.imp();
        imp.set_on_close(closure)
    }
}

#[derive(Debug, Default)]
pub struct OnClose {
    filter: Option<gtk::CustomFilter>,
}

impl OnClose {
    pub fn new(filter: &gtk::CustomFilter) -> Self {
        OnClose {
            filter: Some(filter.clone()),
        }
    }

    pub fn old_new_compare(&self, old: &HashSet<String>, new: &HashSet<String>) {
        let filter_change = Self::determine_filter_change(new, old);

        if let Some(filter_change) = filter_change {
            if let Some(filter) = &self.filter {
                filter.changed(filter_change);
                debug!("Filter change Level {:?}", filter_change);
            }
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
