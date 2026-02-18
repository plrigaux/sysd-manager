use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use gtk::{glib, prelude::*, subclass::prelude::*};
use tracing::debug;

use super::OnClose;

#[derive(Debug, Default, glib::Properties, gtk::CompositeTemplate)]
//#[template(file = "ex_menu_button.ui")]
#[template(resource = "/io/github/plrigaux/sysd-manager/ex_menu_button.ui")]
#[properties(wrapper_type = super::ExMenuButton)]
pub struct ExMenuButtonImpl {
    #[template_child]
    pub toggle: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    pub popover: TemplateChild<gtk::Popover>,

    #[template_child]
    pub button_label: TemplateChild<gtk::Label>,

    #[template_child]
    pub pop_content: TemplateChild<gtk::Box>,

    pub(super) check_boxes: RefCell<HashMap<String, gtk::CheckButton>>,

    pub(super) filter_set: RefCell<HashSet<String>>,

    on_close: RefCell<OnClose>,

    //pub(super) filter: RefCell<gtk::CustomFilter>,
    #[property(get=Self::label, set=Self::set_label)]
    #[allow(dead_code)]
    label: RefCell<String>,
}

#[glib::object_subclass]
impl ObjectSubclass for ExMenuButtonImpl {
    const NAME: &'static str = "ExMenuButton";
    type Type = super::ExMenuButton;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[gtk::template_callbacks]
impl ExMenuButtonImpl {
    #[template_callback]
    fn toggle_toggled(&self, toggle: &gtk::ToggleButton) {
        if toggle.is_active() {
            self.popover.popup();
        }
    }

    #[template_callback(name = "popover_closed")]
    fn unset_toggle(&self) {
        self.toggle.set_active(false);

        let mut new_set: HashSet<String> = HashSet::new();
        let map = self.check_boxes.borrow();

        for (key, check_button) in map.iter() {
            if check_button.is_active() {
                new_set.insert(key.to_owned());
            }
        }

        let old_set = self.filter_set.replace(new_set);
        let on_close = self.on_close.borrow();

        let new_set_ref = &self.filter_set.borrow();
        on_close.old_new_compare(&old_set, new_set_ref);

        debug!("New set {new_set_ref:#?}");
    }

    #[template_callback(name = "clear_filter_selection")]
    fn clear_filter_selection(&self, _button: &gtk::Button) {
        let map = self.check_boxes.borrow();

        for check_button in map.values() {
            check_button.set_active(false);
        }
    }

    pub fn add_item(&self, label: &str) {
        let check = gtk::CheckButton::with_label(label);
        self.pop_content.append(&check);

        let mut map = self.check_boxes.borrow_mut();
        map.insert(label.to_owned(), check.clone());
    }

    fn label(&self) -> String {
        self.button_label.label().to_string()
    }

    fn set_label(&self, label: &str) {
        self.button_label.set_label(label);
    }

    pub fn contains_value(&self, value: Option<&str>) -> bool {
        let set = self.filter_set.borrow();

        if set.is_empty() {
            return true;
        }

        match value {
            Some(v) => set.contains(v),
            None => set.is_empty(),
        }
    }
}

impl ExMenuButtonImpl {
    pub fn set_on_close(&self, closure: OnClose) {
        self.on_close.replace(closure);
    }
}

#[glib::derived_properties]
impl ObjectImpl for ExMenuButtonImpl {
    // Needed for direct subclasses of GtkWidget;
    // Here you need to unparent all direct children
    // of your template.
    fn dispose(&self) {
        self.dispose_template();
    }

    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for ExMenuButtonImpl {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.parent_size_allocate(width, height, baseline);
        self.popover.present();
    }
}

impl BuildableImpl for ExMenuButtonImpl {}

#[cfg(test)]
mod tests {

    /*  use super::*;

    #[test]
    fn test_filter_change() {
        let old_set: HashSet<String> = create_set(&["1", "2", "3"]);
        let new_set: HashSet<String> = create_set(&["1", "2", "3"]);

        assess_filter_change(&new_set, &old_set, None);

        let old_set: HashSet<String> = create_set(&[]);
        let new_set: HashSet<String> = create_set(&[]);

        assess_filter_change(&new_set, &old_set, None);

        let old_set: HashSet<String> = create_set(&[]);
        let new_set: HashSet<String> = create_set(&["1", "2", "3"]);

        assess_filter_change(&new_set, &old_set, Some(FilterChange::MoreStrict));

        let old_set: HashSet<String> = create_set(&["1", "2", "3"]);
        let new_set: HashSet<String> = create_set(&[]);

        assess_filter_change(&new_set, &old_set, Some(FilterChange::LessStrict));

        let old_set: HashSet<String> = create_set(&["1", "2"]);
        let new_set: HashSet<String> = create_set(&["1", "2", "3"]);

        assess_filter_change(&new_set, &old_set, Some(FilterChange::LessStrict));

        let old_set: HashSet<String> = create_set(&["1", "2", "3"]);
        let new_set: HashSet<String> = create_set(&["1", "2"]);

        assess_filter_change(&new_set, &old_set, Some(FilterChange::MoreStrict));

        let old_set: HashSet<String> = create_set(&["1", "2", "3"]);
        let new_set: HashSet<String> = create_set(&["3", "4"]);

        assess_filter_change(&new_set, &old_set, Some(FilterChange::Different));

        let old_set: HashSet<String> = create_set(&["1", "2", "3"]);
        let new_set: HashSet<String> = create_set(&["4", "5"]);

        assess_filter_change(&new_set, &old_set, Some(FilterChange::Different));

        let old_set: HashSet<String> = create_set(&["3", "4"]);
        let new_set: HashSet<String> = create_set(&["1", "2", "3"]);

        assess_filter_change(&new_set, &old_set, Some(FilterChange::Different));

        let old_set: HashSet<String> = create_set(&["4", "5"]);
        let new_set: HashSet<String> = create_set(&["1", "2", "3"]);

        assess_filter_change(&new_set, &old_set, Some(FilterChange::Different));
    }

    fn create_set(list: &[&str]) -> HashSet<String> {
        let set: HashSet<String> = list.iter().map(ToString::to_string).collect();
        set
    }

    fn assess_filter_change(
        new_set: &HashSet<String>,
        old_set: &HashSet<String>,
        expected_filter_change: Option<FilterChange>,
    ) {
        let determined_filter_change = ExMenuButtonImpl::determine_filter_change(new_set, old_set);

        assert_eq!(
            expected_filter_change, determined_filter_change,
            "Old {:?} New {:?} --> Expected {:?} but determined {:?}",
            old_set, new_set, expected_filter_change, determined_filter_change
        );
    } */
}
