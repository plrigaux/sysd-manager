use log::debug;

use crate::{
    systemd::{
        data::UnitInfo,
        enums::{NumMatchType, StrMatchType},
    },
    widget::unit_list::UnitListPanel,
};
use std::{
    any::Any,
    collections::HashSet,
    fmt::{self, Debug},
    hash::Hash,
};
#[derive(Debug, Copy, Clone)]
pub enum UnitPropertyFilterType {
    Text,
    Element,
    NumU64,
    NumI32,
    NumU16,
    NumU32,
    NumI64,
}
pub trait UnitPropertyFilter: Debug {
    fn set_on_change(&mut self, lambda: Box<dyn Fn(bool)>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn ftype(&self) -> UnitPropertyFilterType;

    fn text(&self) -> &str {
        ""
    }

    fn match_type(&self) -> StrMatchType {
        StrMatchType::default()
    }

    fn num_match_type(&self) -> NumMatchType {
        NumMatchType::default()
    }

    fn clear_n_apply_filter(&mut self);
    fn clear_filter(&mut self);
    /*     fn clear_widget_dependancy(&mut self) {
        let lambda = |_: bool| {};
        self.set_on_change(Box::new(lambda));
    } */

    fn is_empty(&self) -> bool;
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
    id: String,
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
        id: &str,
        filter_unit_func: fn(&FilterElementAssessor<T>, &UnitInfo) -> bool,
        unit_list_panel: &UnitListPanel,
    ) -> Self {
        Self {
            filter_elements: Default::default(),
            lambda: Box::new(|_: bool| ()),
            filter_unit_func,
            id: id.to_owned(),
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
                id: self.id.clone(),
            }))
        };

        self.unit_list_panel
            .filter_assessor_change(&self.id, assessor, change_type, false);
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

    fn ftype(&self) -> UnitPropertyFilterType {
        UnitPropertyFilterType::Element
    }
}

pub struct FilterText {
    filter_text: String,
    match_type: StrMatchType,
    lambda: Box<dyn Fn(bool)>,
    filter_unit_func:
        fn(property_assessor: &FilterTextAssessor, unit: &UnitInfo, key: glib::Quark) -> bool,
    id: String,
    unit_list_panel: UnitListPanel,
    key: glib::Quark,
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
        id: &str,
        filter_unit_func: fn(
            property_assessor: &FilterTextAssessor,
            unit: &UnitInfo,
            key: glib::Quark,
        ) -> bool,
        unit_list_panel: &UnitListPanel,
    ) -> Self {
        Self {
            filter_text: Default::default(),
            match_type: StrMatchType::default(),
            lambda: Box::new(|_: bool| ()),
            filter_unit_func,
            id: id.to_owned(),
            unit_list_panel: unit_list_panel.clone(),
            key: glib::Quark::from_str("default"),
        }
    }

    pub fn newq(
        id: &str,
        filter_unit_func: fn(
            property_assessor: &FilterTextAssessor,
            unit: &UnitInfo,
            key: glib::Quark,
        ) -> bool,
        unit_list_panel: &UnitListPanel,
        key: glib::Quark,
    ) -> Self {
        Self {
            filter_text: Default::default(),
            match_type: StrMatchType::default(),
            lambda: Box::new(|_: bool| ()),
            filter_unit_func,
            id: id.to_owned(),
            unit_list_panel: unit_list_panel.clone(),
            key,
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
            Some(Box::new(FilterTextAssessor::new(self)))
        };

        self.unit_list_panel.filter_assessor_change(
            &self.id,
            assessor,
            Some(change_type),
            update_widget,
        );
    }

    pub fn set_filter_match_type(&mut self, match_type: StrMatchType, update_widget: bool) {
        debug!("Match type new {match_type:?} old {:?}", self.match_type);
        if match_type == self.match_type {
            debug!("exit same");
            return;
        }

        let change_type = match (match_type, self.match_type) {
            (StrMatchType::Contains, StrMatchType::StartWith) => gtk::FilterChange::LessStrict,
            (StrMatchType::Contains, StrMatchType::EndWith) => gtk::FilterChange::LessStrict,
            (StrMatchType::StartWith, StrMatchType::Contains) => gtk::FilterChange::MoreStrict,
            (StrMatchType::EndWith, StrMatchType::Contains) => gtk::FilterChange::MoreStrict,

            (_, _) => gtk::FilterChange::Different,
        };

        debug!("change_type {change_type:?}");

        self.match_type = match_type;

        if self.filter_text.is_empty() {
            debug!("exit filter_text empty");
            return;
        }

        let assessor: Option<Box<dyn UnitPropertyAssessor>> =
            Some(Box::new(FilterTextAssessor::new(self)));

        self.unit_list_panel.filter_assessor_change(
            &self.id,
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

    fn match_type(&self) -> StrMatchType {
        self.match_type
    }

    fn clear_n_apply_filter(&mut self) {
        self.filter_text.clear(); //FIXME it does not apply, but might be Ok
        self.match_type = StrMatchType::default();
    }

    fn clear_filter(&mut self) {
        self.clear_n_apply_filter();
        (self.lambda)(true);
    }

    fn is_empty(&self) -> bool {
        self.filter_text.is_empty()
    }

    fn ftype(&self) -> UnitPropertyFilterType {
        UnitPropertyFilterType::Text
    }
}

pub struct FilterNum<T>
where
    T: Debug,
{
    filter_num: T,
    filter_text: String,
    match_type: NumMatchType,
    lambda: Box<dyn Fn(bool)>,
    filter_unit_func: fn(&FilterNumAssessor<T>, &UnitInfo, glib::Quark) -> bool,
    id: String,
    unit_list_panel: UnitListPanel,
    key: glib::Quark,
    ftype: UnitPropertyFilterType,
}

impl<T> fmt::Debug for FilterNum<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterNum")
            .field("filter_num", &self.filter_num)
            .field("id", &self.id)
            .finish()
    }
}

impl<T> FilterNum<T>
where
    T: Debug + Default + PartialEq + PartialOrd + Copy + 'static,
{
    pub fn new(
        id: &str,
        filter_unit_func: fn(&FilterNumAssessor<T>, &UnitInfo, glib::Quark) -> bool,
        unit_list_panel: &UnitListPanel,
        key: glib::Quark,
        ftype: UnitPropertyFilterType,
    ) -> Self {
        Self {
            filter_num: Default::default(),
            filter_text: Default::default(),
            match_type: NumMatchType::default(),
            lambda: Box::new(|_: bool| ()),
            filter_unit_func,
            id: id.to_string(),
            unit_list_panel: unit_list_panel.clone(),
            key,
            ftype,
        }
    }

    pub fn set_filter_elem(&mut self, f_element: T, update_widget: bool) {
        if f_element == self.filter_num {
            return;
        }
        /*
        self.unit_list_panel.filter_assessor_change(
            self.id,
            assessor,
            Some(change_type),
            update_widget,
        ); */
    }

    pub fn set_filter_match_type(&mut self, match_type: NumMatchType, update_widget: bool) {
        debug!("Match type new {match_type:?} old {:?}", self.match_type);
        if match_type == self.match_type {
            debug!("exit same");
            return;
        }

        let change_type = match (self.match_type, match_type) {
            (NumMatchType::Equals, NumMatchType::GreaterEquals) => gtk::FilterChange::LessStrict,
            (NumMatchType::Equals, NumMatchType::SmallerEquals) => gtk::FilterChange::LessStrict,
            (NumMatchType::SmallerEquals, NumMatchType::Smaller) => gtk::FilterChange::MoreStrict,
            (NumMatchType::GreaterEquals, NumMatchType::Greater) => gtk::FilterChange::MoreStrict,
            (NumMatchType::Smaller, NumMatchType::SmallerEquals) => gtk::FilterChange::LessStrict,
            (NumMatchType::Greater, NumMatchType::GreaterEquals) => gtk::FilterChange::LessStrict,
            (_, _) => gtk::FilterChange::Different,
        };

        debug!("change_type {change_type:?}");

        self.match_type = match_type;

        //FIXME self.filter_text.is_empty()

        /*         if self.filter_text.is_empty() {
            debug!("exit filter_text empty");
            return;
        } */

        let assessor: Option<Box<dyn UnitPropertyAssessor>> =
            Some(Box::new(FilterNumAssessor::new(self)));

        self.unit_list_panel.filter_assessor_change(
            &self.id,
            assessor,
            Some(change_type),
            update_widget,
        );
    }
}

impl<T> UnitPropertyFilter for FilterNum<T>
where
    T: Debug + ToString + 'static,
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

    fn text(&self) -> &str {
        &self.filter_text
    }

    fn match_type(&self) -> StrMatchType {
        StrMatchType::default()
    }

    fn num_match_type(&self) -> NumMatchType {
        self.match_type
    }

    fn clear_n_apply_filter(&mut self) {
        // self.filter_text.clear(); //FIXME it does not apply, but might be Ok
        self.match_type = NumMatchType::default();
    }

    fn clear_filter(&mut self) {
        self.clear_n_apply_filter();
        (self.lambda)(true);
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn ftype(&self) -> UnitPropertyFilterType {
        self.ftype
    }
}

pub trait UnitPropertyAssessor: core::fmt::Debug {
    fn filter_unit(&self, unit: &UnitInfo) -> bool;
    //  fn filter_unit_value(&self, unit_value: &str) -> bool;
    fn id(&self) -> &str;
    fn text(&self) -> &str {
        ""
    }

    fn match_type(&self) -> StrMatchType {
        StrMatchType::default()
    }
}

#[derive(Debug)]
pub struct FilterElementAssessor<T>
where
    T: Eq + Hash + Debug,
{
    filter_elements: HashSet<T>,
    filter_unit_func: fn(&FilterElementAssessor<T>, &UnitInfo) -> bool,
    id: String,
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

    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug)]
pub struct FilterTextAssessor {
    filter_text: String,
    filter_unit_func: fn(&FilterTextAssessor, &UnitInfo, glib::Quark) -> bool,
    id: String,
    key: glib::Quark,
    pub(crate) filter_unit_value_func:
        fn(filter_text: &FilterTextAssessor, unit_value: Option<&str>) -> bool,
}

impl FilterTextAssessor {
    fn new(filter_text: &FilterText) -> Self {
        let filter_unit_value_func =
            match (filter_text.filter_text.is_empty(), filter_text.match_type) {
                (true, _) => Self::filter_unit_value_func_empty,
                (false, StrMatchType::Contains) => Self::filter_unit_value_func_contains,
                (false, StrMatchType::StartWith) => Self::filter_unit_value_func_start_with,
                (false, StrMatchType::EndWith) => Self::filter_unit_value_func_end_with,
            };

        FilterTextAssessor {
            filter_text: filter_text.filter_text.clone(),
            filter_unit_func: filter_text.filter_unit_func,
            id: filter_text.id.clone(),
            filter_unit_value_func,
            key: filter_text.key,
        }
    }

    fn filter_unit_value_func_empty(&self, _unit_value: Option<&str>) -> bool {
        true
    }

    fn filter_unit_value_func_contains(&self, unit_value: Option<&str>) -> bool {
        if let Some(unit_value) = unit_value {
            unit_value.contains(&self.filter_text)
        } else {
            false
        }
    }

    fn filter_unit_value_func_start_with(&self, unit_value: Option<&str>) -> bool {
        if let Some(unit_value) = unit_value {
            unit_value.starts_with(&self.filter_text)
        } else {
            false
        }
    }

    fn filter_unit_value_func_end_with(&self, unit_value: Option<&str>) -> bool {
        if let Some(unit_value) = unit_value {
            unit_value.ends_with(&self.filter_text)
        } else {
            false
        }
    }
}

impl UnitPropertyAssessor for FilterTextAssessor {
    fn filter_unit(&self, unit: &UnitInfo) -> bool {
        (self.filter_unit_func)(self, unit, self.key)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn text(&self) -> &str {
        &self.filter_text
    }
}

#[derive(Debug)]
pub struct FilterNumAssessor<T>
where
    T: Debug,
{
    filter_num: T,
    //match_type: MatchType, //TODO make distintive struct to avoid runtime if
    filter_unit_func: fn(&FilterNumAssessor<T>, &UnitInfo, glib::Quark) -> bool,
    id: String,
    pub(crate) filter_unit_value_func:
        fn(filter_text: &FilterNumAssessor<T>, unit_value: Option<T>) -> bool,
    key: glib::Quark,
}

impl<T> FilterNumAssessor<T>
where
    T: Debug + Copy + PartialEq + PartialOrd,
{
    fn new(filter_num: &FilterNum<T>) -> Self {
        let filter_unit_value_func =
            match (filter_num.filter_text.is_empty(), filter_num.match_type) {
                (true, _) => Self::filter_unit_value_func_empty,

                (false, NumMatchType::Equals) => Self::filter_unit_value_func_equals,
                (false, NumMatchType::Greater) => Self::filter_unit_value_func_greater,
                (false, NumMatchType::Smaller) => Self::filter_unit_value_func_smaller,
                (false, NumMatchType::GreaterEquals) => Self::filter_unit_value_func_greater_equals,
                (false, NumMatchType::SmallerEquals) => Self::filter_unit_value_func_smaller_equals,
            };

        Self {
            filter_num: filter_num.filter_num,
            filter_unit_func: filter_num.filter_unit_func,
            id: filter_num.id.clone(),
            filter_unit_value_func,
            key: filter_num.key,
        }
    }

    fn filter_unit_value_func_empty(&self, _unit_value: Option<T>) -> bool {
        true
    }

    fn filter_unit_value_func_equals(&self, unit_value: Option<T>) -> bool {
        unit_value == Some(self.filter_num)
    }

    fn filter_unit_value_func_smaller(&self, unit_value: Option<T>) -> bool {
        Some(self.filter_num) > unit_value
    }

    fn filter_unit_value_func_smaller_equals(&self, unit_value: Option<T>) -> bool {
        Some(self.filter_num) >= unit_value
    }

    fn filter_unit_value_func_greater(&self, unit_value: Option<T>) -> bool {
        Some(self.filter_num) > unit_value
    }

    fn filter_unit_value_func_greater_equals(&self, unit_value: Option<T>) -> bool {
        Some(self.filter_num) >= unit_value
    }
}

impl<T> UnitPropertyAssessor for FilterNumAssessor<T>
where
    T: Debug,
{
    fn filter_unit(&self, unit: &UnitInfo) -> bool {
        (self.filter_unit_func)(self, unit, self.key)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn text(&self) -> &str {
        ""
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_comp() {
        let a = 4;
        let b = None;

        let c = Some(a) < b;

        println!("c {c}");

        let d = Some(a) > b;

        println!("d {d}");
    }
}
