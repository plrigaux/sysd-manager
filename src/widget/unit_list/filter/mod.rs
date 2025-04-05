mod imp;

use std::{any::Any, collections::HashSet};

use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::{
    FilterChange,
    glib::{self},
};

use crate::systemd::{
    data::UnitInfo,
    enums::{EnablementStatus, StartStopMode},
};

use super::UnitListPanel;

// ANCHOR: mod
glib::wrapper! {
    pub struct UnitListFilterWindow(ObjectSubclass<imp::UnitListFilterWindowImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl UnitListFilterWindow {
    pub fn new(selected_filter: Option<String>, unit_list_panel: &UnitListPanel) -> Self {
        let obj: UnitListFilterWindow = glib::Object::builder()
            .property("selected", selected_filter)
            .build();

        obj.imp().unit_list_panel.set(unit_list_panel.clone());

        obj
    }

    pub fn get_filter(&self, unit_list_panel: &UnitListPanel) {
        self.imp().get_filter(unit_list_panel)
    }
}

pub trait UnitPropertyFilter {
    fn filter_unit(&self, unit: &UnitInfo) -> bool;
    fn set_on_change(&mut self, lambda: Box<dyn Fn(bool)>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn set_filter_elem(&mut self, f_element: &str, set_element: bool);
}

pub struct FilterElem {
    filter_elements: HashSet<String>,
    lambda: Box<dyn Fn(bool)>,
    filter_unit_func: fn(&UnitInfo, &HashSet<String>) -> bool,
    id: u8,
    unit_list_panel: UnitListPanel,
}

fn filter_unit_func_default(_: &UnitInfo, _: &FilterElem) -> bool {
    true
}

impl FilterElem {
    fn new(
        id: u8,
        filter_unit_func: fn(&UnitInfo, &HashSet<String>) -> bool,
        unit_list_panel: &UnitListPanel,
    ) -> Self {
        Self {
            filter_elements: Default::default(),
            lambda: Box::new(|_: bool| ()),
            filter_unit_func,
            id,
            unit_list_panel: unit_list_panel.clone(),
        }
    }

    fn change_type(has_changed: bool, set_element: bool) -> Option<FilterChange> {
        match (has_changed, set_element) {
            (true, true) => Some(FilterChange::MoreStrict),
            (true, false) => Some(FilterChange::LessStrict),
            (false, _) => None,
        }
    }
}

impl UnitPropertyFilter for FilterElem {
    fn filter_unit(&self, unit: &UnitInfo) -> bool {
        (self.filter_unit_func)(unit, &self.filter_elements)
    }

    fn set_on_change(&mut self, lambda: Box<dyn Fn(bool)>) {
        self.lambda = lambda
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn set_filter_elem(&mut self, f_element: &str, set_element: bool) {
        let old_is_empty = self.filter_elements.is_empty();

        let has_changed = if set_element {
            self.filter_elements.insert(f_element.to_owned())
        } else {
            self.filter_elements.remove(f_element)
        };

        let _ = FilterElem::change_type(has_changed, set_element);

        let new_is_empty = self.filter_elements.is_empty();
        if old_is_empty != new_is_empty {
            (self.lambda)(new_is_empty);
        }

        if new_is_empty {
            //rem
        } else if old_is_empty {
            //add
        }
    }
}

pub struct FilterText {
    filter_text: String,
    lambda: Box<dyn Fn(bool)>,
    filter_unit_func: fn(unit: &UnitInfo, filter_text: &str) -> bool,
    id: u8,
    unit_list_panel: UnitListPanel,
}

impl FilterText {
    fn new(
        id: u8,
        filter_unit_func: fn(&UnitInfo, &str) -> bool,
        unit_list_panel: &UnitListPanel,
    ) -> Self {
        Self {
            filter_text: Default::default(),
            lambda: Box::new(|_: bool| ()),
            filter_unit_func,
            id,
            unit_list_panel: unit_list_panel.clone(),
        }
    }

    fn change_type(has_changed: bool, set_element: bool) -> Option<FilterChange> {
        match (has_changed, set_element) {
            (true, true) => Some(FilterChange::MoreStrict),
            (true, false) => Some(FilterChange::LessStrict),
            (false, _) => None,
        }
    }

    fn set_on_change(&mut self, lambda: impl Fn(bool) + 'static) {
        self.lambda = Box::new(lambda)
    }
}

impl UnitPropertyFilter for FilterText {
    fn filter_unit(&self, unit: &UnitInfo) -> bool {
        (self.filter_unit_func)(unit, &self.filter_text)
    }

    fn set_on_change(&mut self, lambda: Box<dyn Fn(bool)>) {
        self.lambda = lambda
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn set_filter_elem(&mut self, f_element: &str, _: bool) {
        if f_element == self.filter_text {
            return;
        }

        let old_is_empty = self.filter_text.is_empty();
        self.filter_text.replace_range(.., f_element);

        let new_is_empty = self.filter_text.is_empty();
        if old_is_empty != new_is_empty {
            (self.lambda)(new_is_empty);
        }
    }
}

fn filter_active_state(unit: &UnitInfo, filter_elements: &HashSet<String>) -> bool {
    let active_state = unit.active_state();
    filter_elements.contains(active_state.as_str())
}

fn filter_unit_type(unit: &UnitInfo, filter_elements: &HashSet<String>) -> bool {
    let unit_type = unit.unit_type();
    filter_elements.contains(unit_type.as_str())
}

fn filter_enable_status(unit: &UnitInfo, filter_elements: &HashSet<String>) -> bool {
    let enable_status: EnablementStatus = unit.enable_status().into();
    filter_elements.contains(enable_status.as_str())
}

fn filter_unit_name(unit: &UnitInfo, filter_text: &str) -> bool {
    let name = unit.display_name();

    if name.is_empty() {
        true
    } else {
        unit.display_name().contains(filter_text)
    }
}

fn filter_unit_description(unit: &UnitInfo, filter_text: &str) -> bool {
    let name = unit.description();

    if name.is_empty() {
        true
    } else {
        unit.display_name().contains(filter_text)
    }
}
