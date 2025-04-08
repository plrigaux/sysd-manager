mod imp;
use std::{any::Any, collections::HashSet, hash::Hash};

use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::{
    FilterChange,
    glib::{self},
};

use super::UnitListPanel;
use crate::systemd::{
    data::UnitInfo,
    enums::{ActiveState, EnablementStatus},
};
use std::fmt::Debug;

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

    fn text(&self) -> &str {
        ""
    }
    fn clear_filter(&mut self);
    fn clear_widget_dependancy(&mut self) {
        let lambda = |_: bool| {};
        self.set_on_change(Box::new(lambda));
    }

    fn is_empty(&self) -> bool;
}

/* pub fn get_filter_text(prop_filter: &dyn UnitPropertyFilter) -> &FilterText {
    prop_filter
        .as_any()
        .downcast_ref::<FilterText>()
        .expect("downcast_mut to FilterText")
}

pub fn get_filter_text_mut(prop_filter: &mut dyn UnitPropertyFilter) -> &mut FilterText {
    prop_filter
        .as_any_mut()
        .downcast_mut::<FilterText>()
        .expect("downcast_mut to FilterText")
}
 */
pub fn get_filter_element<T>(prop_filter: &dyn UnitPropertyFilter) -> &FilterElement<T>
where
    T: Eq + Hash + Debug + 'static,
{
    prop_filter
        .as_any()
        .downcast_ref::<FilterElement<T>>()
        .expect("downcast_mut to FilterElement")
}

pub fn get_filter_element_mut<T>(prop_filter: &mut dyn UnitPropertyFilter) -> &mut FilterElement<T>
where
    T: Eq + Hash + Debug + 'static,
{
    prop_filter
        .as_any_mut()
        .downcast_mut::<FilterElement<T>>()
        .expect("downcast_mut to FilterElement")
}

pub struct FilterElement<T>
where
    T: Eq + Hash + Debug,
{
    filter_elements: HashSet<T>,
    lambda: Box<dyn Fn(bool)>,
    filter_unit_func: fn(&FilterElementAssessor<T>, &UnitInfo) -> bool,
    id: u8,
    unit_list_panel: UnitListPanel,
}

impl<T> FilterElement<T>
where
    T: Eq + Hash + Debug + Clone + 'static,
{
    pub fn new(
        id: u8,
        filter_unit_func: fn(&FilterElementAssessor<T>, &UnitInfo) -> bool,
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

    fn contains(&self, value: &T) -> bool {
        self.filter_elements.contains(value)
    }

    fn set_filter_elem(&mut self, f_element: T, set_element: bool) {
        let old_is_empty = self.filter_elements.is_empty();

        let has_changed = if set_element {
            self.filter_elements.insert(f_element)
        } else {
            self.filter_elements.remove(&f_element)
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
}

impl<T> UnitPropertyFilter for FilterElement<T>
where
    T: Eq + Hash + Debug + 'static,
{
    fn set_on_change(&mut self, lambda: Box<dyn Fn(bool)>) {
        self.lambda = lambda
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
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
    filter_unit_func: fn(property_assessor: &FilterTextAssessor, unit: &UnitInfo) -> bool,
    id: u8,
    unit_list_panel: UnitListPanel,
}

impl FilterText {
    pub fn new(
        id: u8,
        filter_unit_func: fn(property_assessor: &FilterTextAssessor, unit: &UnitInfo) -> bool,
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

    pub fn set_filter_elem(&mut self, f_element: &str) {
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
    //  fn filter_unit_value(&self, unit_value: &str) -> bool;
    fn id(&self) -> u8;
    fn text(&self) -> &str {
        ""
    }
}

#[derive(Debug)]
pub struct FilterElementAssessor<T>
where
    T: Eq + Hash + Debug,
{
    filter_elements: HashSet<T>,
    filter_unit_func: fn(&FilterElementAssessor<T>, &UnitInfo) -> bool,
    id: u8,
}

impl<T> FilterElementAssessor<T>
where
    T: Eq + Hash + Debug,
{
    fn filter_unit_value(&self, unit_value: &T) -> bool {
        self.filter_elements.contains(unit_value)
    }
}

impl<T> UnitPropertyAssessor for FilterElementAssessor<T>
where
    T: Eq + Hash + Debug,
{
    fn filter_unit(&self, unit: &UnitInfo) -> bool {
        (self.filter_unit_func)(self, unit)
    }

    fn id(&self) -> u8 {
        self.id
    }
}

#[derive(Debug)]
pub struct FilterTextAssessor {
    filter_text: String,
    filter_unit_func: fn(&FilterTextAssessor, &UnitInfo) -> bool,
    id: u8,
}

impl FilterTextAssessor {
    fn filter_unit_value(&self, unit_value: &str) -> bool {
        if self.filter_text.is_empty() {
            true
        } else {
            unit_value.contains(&self.filter_text)
        }
    }
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
}

pub fn filter_load_state(
    property_assessor: &FilterElementAssessor<String>,
    unit: &UnitInfo,
) -> bool {
    property_assessor.filter_unit_value(&unit.load_state())
}

pub fn filter_active_state(
    property_assessor: &FilterElementAssessor<ActiveState>,
    unit: &UnitInfo,
) -> bool {
    let active_state = unit.active_state();
    property_assessor.filter_unit_value(&active_state)
}

pub fn filter_unit_type(
    property_assessor: &FilterElementAssessor<String>,
    unit: &UnitInfo,
) -> bool {
    property_assessor.filter_unit_value(&unit.unit_type())
}

pub fn filter_enable_status(
    property_assessor: &FilterElementAssessor<EnablementStatus>,
    unit: &UnitInfo,
) -> bool {
    let enable_status: EnablementStatus = unit.enable_status().into();
    property_assessor.filter_unit_value(&enable_status)
}

pub fn filter_unit_name(property_assessor: &FilterTextAssessor, unit: &UnitInfo) -> bool {
    let name = unit.display_name();
    property_assessor.filter_unit_value(&name)
}

pub fn filter_unit_description(property_assessor: &FilterTextAssessor, unit: &UnitInfo) -> bool {
    let description = unit.description();
    property_assessor.filter_unit_value(&description)
}
