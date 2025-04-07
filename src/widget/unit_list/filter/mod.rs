mod imp;

use std::{any::Any, collections::HashSet};

use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::{
    FilterChange,
    glib::{self},
};

use crate::systemd::{data::UnitInfo, enums::EnablementStatus};

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
        //   obj.set_default_width(300);
        let _ = obj.imp().unit_list_panel.set(unit_list_panel.clone());

        obj
    }

    pub fn construct_filter_dialog(&self) {
        self.imp().get_filter()
    }
}

pub trait UnitPropertyFilter {
    fn set_on_change(&mut self, lambda: Box<dyn Fn(bool)>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn set_filter_elem(&mut self, f_element: &str, set_element: bool);
    fn text(&self) -> &str {
        ""
    }
    fn clear_filter(&mut self);
    fn clear_widget_dependancy(&mut self) {
        let lambda = |_: bool| {};
        self.set_on_change(Box::new(lambda));
    }
    fn contains(&self, _value: &str) -> bool {
        false
    }
    fn is_empty(&self) -> bool;
}

pub struct FilterElem {
    filter_elements: HashSet<String>,
    lambda: Box<dyn Fn(bool)>,
    filter_unit_func: fn(&dyn UnitPropertyAssessor, &UnitInfo) -> bool,
    id: u8,
    unit_list_panel: UnitListPanel,
}

impl FilterElem {
    pub fn new(
        id: u8,
        filter_unit_func: fn(&dyn UnitPropertyAssessor, &UnitInfo) -> bool,
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
}

impl UnitPropertyFilter for FilterElem {
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

        if !has_changed {
            return;
        }

        let new_is_empty = self.filter_elements.is_empty();

        let change_type = match (set_element, old_is_empty, new_is_empty) {
            (true, true, _) => Some(FilterChange::MoreStrict),
            (true, false, _) => Some(FilterChange::LessStrict),
            (false, _, false) => Some(FilterChange::MoreStrict),
            (false, _, true) => Some(FilterChange::LessStrict),
        };

        if old_is_empty != new_is_empty {
            (self.lambda)(new_is_empty);
        }

        let assessor: Option<Box<dyn UnitPropertyAssessor>> = if new_is_empty {
            None
        } else {
            Some(Box::new(FilterElementAssessor {
                filter_elements: self.filter_elements.clone(),
                filter_unit_func: self.filter_unit_func,
                id: self.id,
            }))
        };

        self.unit_list_panel
            .filter_assessor_change(self.id, assessor, change_type);
    }

    fn contains(&self, value: &str) -> bool {
        self.filter_elements.contains(value)
    }

    fn clear_filter(&mut self) {
        self.filter_elements.clear();
    }

    fn is_empty(&self) -> bool {
        self.filter_elements.is_empty()
    }
}

pub struct FilterText {
    filter_text: String,
    lambda: Box<dyn Fn(bool)>,
    filter_unit_func: fn(property_assessor: &dyn UnitPropertyAssessor, unit: &UnitInfo) -> bool,
    id: u8,
    unit_list_panel: UnitListPanel,
}

impl FilterText {
    pub fn new(
        id: u8,
        filter_unit_func: fn(property_assessor: &dyn UnitPropertyAssessor, unit: &UnitInfo) -> bool,
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
}

impl UnitPropertyFilter for FilterText {
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
        let new_is_empty = f_element.is_empty();

        let change_type = if new_is_empty {
            gtk::FilterChange::LessStrict
        } else if f_element.len() > self.filter_text.len() && f_element.contains(&self.filter_text)
        {
            gtk::FilterChange::MoreStrict
        } else if f_element.len() < self.filter_text.len() && self.filter_text.contains(f_element) {
            gtk::FilterChange::LessStrict
        } else {
            gtk::FilterChange::Different
        };

        self.filter_text.replace_range(.., f_element);

        if old_is_empty != new_is_empty {
            (self.lambda)(new_is_empty);
        }

        let assessor: Option<Box<dyn UnitPropertyAssessor>> = if new_is_empty {
            None
        } else {
            Some(Box::new(FilterTextAssessor {
                filter_text: self.filter_text.clone(),
                filter_unit_func: self.filter_unit_func,
                id: self.id,
            }))
        };

        self.unit_list_panel
            .filter_assessor_change(self.id, assessor, Some(change_type));
    }

    fn text(&self) -> &str {
        &self.filter_text
    }

    fn clear_filter(&mut self) {
        self.filter_text.clear();
    }

    fn is_empty(&self) -> bool {
        self.filter_text.is_empty()
    }
}

pub trait UnitPropertyAssessor: core::fmt::Debug {
    fn filter_unit(&self, unit: &UnitInfo) -> bool;
    fn filter_unit_value(&self, unit_value: &str) -> bool;
    fn id(&self) -> u8;
    fn text(&self) -> &str {
        ""
    }
}

#[derive(Debug)]
pub struct FilterElementAssessor {
    filter_elements: HashSet<String>,
    filter_unit_func: fn(&dyn UnitPropertyAssessor, &UnitInfo) -> bool,
    id: u8,
}

impl UnitPropertyAssessor for FilterElementAssessor {
    fn filter_unit(&self, unit: &UnitInfo) -> bool {
        (self.filter_unit_func)(self, unit)
    }

    fn id(&self) -> u8 {
        self.id
    }

    fn filter_unit_value(&self, unit_value: &str) -> bool {
        self.filter_elements.contains(unit_value)
    }
}

#[derive(Debug)]
pub struct FilterTextAssessor {
    filter_text: String,
    filter_unit_func: fn(&dyn UnitPropertyAssessor, &UnitInfo) -> bool,
    id: u8,
}

impl UnitPropertyAssessor for FilterTextAssessor {
    fn filter_unit(&self, unit: &UnitInfo) -> bool {
        (self.filter_unit_func)(self, unit)
    }

    fn id(&self) -> u8 {
        self.id
    }

    fn text(&self) -> &str {
        &self.filter_text
    }

    fn filter_unit_value(&self, unit_value: &str) -> bool {
        if self.filter_text.is_empty() {
            true
        } else {
            unit_value.contains(&self.filter_text)
        }
    }
}

pub fn filter_load_state(property_assessor: &dyn UnitPropertyAssessor, unit: &UnitInfo) -> bool {
    property_assessor.filter_unit_value(&unit.load_state())
}

pub fn filter_active_state(property_assessor: &dyn UnitPropertyAssessor, unit: &UnitInfo) -> bool {
    let active_state = unit.active_state();
    property_assessor.filter_unit_value(active_state.as_str())
}

pub fn filter_unit_type(property_assessor: &dyn UnitPropertyAssessor, unit: &UnitInfo) -> bool {
    property_assessor.filter_unit_value(&unit.unit_type())
}

pub fn filter_enable_status(property_assessor: &dyn UnitPropertyAssessor, unit: &UnitInfo) -> bool {
    let enable_status: EnablementStatus = unit.enable_status().into();
    property_assessor.filter_unit_value(enable_status.as_str())
}

pub fn filter_unit_name(property_assessor: &dyn UnitPropertyAssessor, unit: &UnitInfo) -> bool {
    let name = unit.display_name();
    property_assessor.filter_unit_value(&name)
}

pub fn filter_unit_description(
    property_assessor: &dyn UnitPropertyAssessor,
    unit: &UnitInfo,
) -> bool {
    let description = unit.description();
    property_assessor.filter_unit_value(&description)
}
