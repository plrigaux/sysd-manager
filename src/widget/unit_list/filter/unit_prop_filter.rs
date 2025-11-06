use log::debug;

use crate::{
    systemd::{data::UnitInfo, enums::MatchType},
    widget::unit_list::UnitListPanel,
};
use std::{
    any::Any,
    collections::HashSet,
    fmt::{self, Debug},
    hash::Hash,
};

pub trait UnitPropertyFilter: Debug {
    fn set_on_change(&mut self, lambda: Box<dyn Fn(bool)>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn text(&self) -> &str {
        ""
    }

    fn clear_n_apply_filter(&mut self);
    fn clear_filter(&mut self);
    fn clear_widget_dependancy(&mut self) {
        let lambda = |_: bool| {};
        self.set_on_change(Box::new(lambda));
    }

    fn is_empty(&self) -> bool;

    fn match_type(&self) -> MatchType {
        MatchType::default()
    }
}

pub fn get_filter_element<T>(prop_filter: &dyn UnitPropertyFilter) -> &FilterElement<T>
where
    T: Eq + Hash + Debug + 'static,
{
    match prop_filter.as_any().downcast_ref::<FilterElement<T>>() {
        Some(a) => a,
        None => {
            panic!("Type of prop_filter, Expect: FilterElement",);
        }
    }
    //.expect("downcast_mut to FilterElement")
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

impl<T> fmt::Debug for FilterElement<T>
where
    T: Eq + Hash + Debug + Clone + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = std::any::TypeId::of::<T>();

        f.debug_struct("FilterElement")
            .field("SUB_TYPE", &t)
            .field("filter_elements", &self.filter_elements)
            .field("id", &self.id)
            .finish()
    }
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

    pub(crate) fn elements(&self) -> &HashSet<T> {
        &self.filter_elements
    }

    pub(crate) fn contains(&self, value: &T) -> bool {
        self.filter_elements.contains(value)
    }

    pub(crate) fn set_filter_elem(&mut self, f_element: T, add_or_remove: bool) {
        let old_is_empty = self.filter_elements.is_empty();

        let has_changed = if add_or_remove {
            self.filter_elements.insert(f_element)
        } else {
            self.filter_elements.remove(&f_element)
        };

        if !has_changed {
            return;
        }

        let new_is_empty = self.filter_elements.is_empty();

        let change_type = match (add_or_remove, old_is_empty, new_is_empty) {
            (true, true, _) => Some(gtk::FilterChange::MoreStrict),
            (true, false, _) => Some(gtk::FilterChange::LessStrict),
            (false, _, false) => Some(gtk::FilterChange::MoreStrict),
            (false, _, true) => Some(gtk::FilterChange::LessStrict),
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
            .filter_assessor_change(self.id, assessor, change_type, false);
    }
}

impl<T> UnitPropertyFilter for FilterElement<T>
where
    T: Eq + Hash + Debug + Clone + 'static,
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

    fn clear_n_apply_filter(&mut self) {
        let set = self.filter_elements.clone();
        for f_element in set {
            FilterElement::set_filter_elem(self, f_element, false);
        }
    }

    fn clear_filter(&mut self) {
        self.filter_elements.clear();
        (self.lambda)(true);
    }

    fn is_empty(&self) -> bool {
        self.filter_elements.is_empty()
    }
}

pub struct FilterText {
    filter_text: String,
    match_type: MatchType,
    lambda: Box<dyn Fn(bool)>,
    filter_unit_func: fn(property_assessor: &FilterTextAssessor, unit: &UnitInfo) -> bool,
    id: u8,
    unit_list_panel: UnitListPanel,
}

impl fmt::Debug for FilterText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterText")
            .field("filter_text", &self.filter_text)
            .field("id", &self.id)
            .finish()
    }
}

impl FilterText {
    pub fn new(
        id: u8,
        filter_unit_func: fn(property_assessor: &FilterTextAssessor, unit: &UnitInfo) -> bool,
        unit_list_panel: &UnitListPanel,
    ) -> Self {
        Self {
            filter_text: Default::default(),
            match_type: MatchType::default(),
            lambda: Box::new(|_: bool| ()),
            filter_unit_func,
            id,
            unit_list_panel: unit_list_panel.clone(),
        }
    }

    pub fn set_filter_elem(&mut self, f_element: &str, update_widget: bool) {
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
                match_type: self.match_type,
                filter_unit_func: self.filter_unit_func,
                id: self.id,
            }))
        };

        self.unit_list_panel.filter_assessor_change(
            self.id,
            assessor,
            Some(change_type),
            update_widget,
        );
    }

    pub fn set_filter_match_type(&mut self, match_type: MatchType, update_widget: bool) {
        debug!("Match type new {match_type:?} old {:?}", self.match_type);
        if match_type == self.match_type {
            debug!("exit same");
            return;
        }

        let change_type = match (match_type, self.match_type) {
            (MatchType::Contains, MatchType::StartWith) => gtk::FilterChange::LessStrict,
            (MatchType::Contains, MatchType::EndWith) => gtk::FilterChange::LessStrict,
            (MatchType::StartWith, MatchType::Contains) => gtk::FilterChange::MoreStrict,
            (MatchType::EndWith, MatchType::Contains) => gtk::FilterChange::MoreStrict,

            (_, _) => gtk::FilterChange::Different,
        };

        debug!("change_type {change_type:?}");

        self.match_type = match_type;

        if self.filter_text.is_empty() {
            debug!("exit filter_text empty");
            return;
        }

        let assessor: Option<Box<dyn UnitPropertyAssessor>> = Some(Box::new(FilterTextAssessor {
            filter_text: self.filter_text.clone(),
            match_type: self.match_type,
            filter_unit_func: self.filter_unit_func,
            id: self.id,
        }));

        self.unit_list_panel.filter_assessor_change(
            self.id,
            assessor,
            Some(change_type),
            update_widget,
        );
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

    fn clear_n_apply_filter(&mut self) {
        self.filter_text.clear(); //FIXME it does not apply
    }

    fn clear_filter(&mut self) {
        self.filter_text.clear();
        (self.lambda)(true);
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

    fn match_type(&self) -> MatchType {
        MatchType::default()
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
    pub(crate) fn filter_unit_value(&self, unit_value: &T) -> bool {
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
    match_type: MatchType, //TODO make distintive struct to avoid runtime if
    filter_unit_func: fn(&FilterTextAssessor, &UnitInfo) -> bool,
    id: u8,
}

impl FilterTextAssessor {
    pub(crate) fn filter_unit_value(&self, unit_value: &str) -> bool {
        if self.filter_text.is_empty() {
            true
        } else {
            match self.match_type {
                MatchType::Contains => unit_value.contains(&self.filter_text),
                MatchType::StartWith => unit_value.starts_with(&self.filter_text),
                MatchType::EndWith => unit_value.ends_with(&self.filter_text),
            }
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

    fn match_type(&self) -> MatchType {
        self.match_type
    }
}
