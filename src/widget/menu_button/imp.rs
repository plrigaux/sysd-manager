use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use gtk::{glib, prelude::*, subclass::prelude::*, FilterChange};
use log::info;

#[derive(Debug, Default, gtk::CompositeTemplate)]
//#[template(file = "ex_menu_button.ui")]
#[template(resource = "/org/tool/sysd/manager/ex_menu_button.ui")]
pub struct ExMenuButton {
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

    pub(super) filter: RefCell<gtk::CustomFilter>,
}

#[glib::object_subclass]
impl ObjectSubclass for ExMenuButton {
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
impl ExMenuButton {
    #[template_callback]
    fn toggle_toggled(&self, toggle: &gtk::ToggleButton) {
        if toggle.is_active() {
            self.popover.popup();
        }
    }

    #[template_callback(name = "popover_closed")]
    fn unset_toggle(&self) {
        self.toggle.set_active(false);

        let filter_change: Option<FilterChange>;
        let mut new_set: HashSet<String> = HashSet::new();
        {
            let map = self.check_boxes.borrow();
            let old_set = self.filter_set.borrow();

            for (key, check_button) in map.iter() {
                if check_button.is_active() {
                    new_set.insert(key.to_owned());
                }
            }

            if old_set.is_empty() && !new_set.is_empty() {
                filter_change = Some(FilterChange::MoreStrict);
            } else if !old_set.is_empty() && new_set.is_empty() {
                filter_change = Some(FilterChange::LessStrict);
            } else if old_set.len() == new_set.len() {
                filter_change = if old_set.iter().all(|item| new_set.contains(item)) {
                    None
                } else {
                    Some(FilterChange::Different)
                };
            } else if old_set.len() > new_set.len() {
                filter_change = if new_set.iter().all(|item| old_set.contains(item)) {
                    Some(FilterChange::MoreStrict)
                } else {
                    Some(FilterChange::Different)
                };
            } else {
                filter_change = if old_set.iter().all(|item| new_set.contains(item)) {
                    Some(FilterChange::LessStrict)
                } else {
                    Some(FilterChange::Different)
                };
            }
        }

        self.filter_set.replace(new_set);

        info!("Filter Level {:?}", filter_change);

        if let Some(fc) = filter_change {
            self.filter.borrow().changed(fc)
        }
    }

    #[template_callback(name = "clear_filter_selection")]
    fn clear_filter_selection(&self, _button: &gtk::Button) {
        let map = self.check_boxes.borrow();

        for check_button in map.values().into_iter() {
            check_button.set_active(false);
        }
    }

    pub fn add_item(&self, label: &str) {
        let check = gtk::CheckButton::with_label(label);
        self.pop_content.append(&check);

        let mut map = self.check_boxes.borrow_mut();
        map.insert(label.to_owned(), check.clone());
    }
}

impl ObjectImpl for ExMenuButton {
    // Needed for direct subclasses of GtkWidget;
    // Here you need to unparent all direct children
    // of your template.
    fn dispose(&self) {
        self.dispose_template();
    }

    fn constructed(&self) {}
}

impl WidgetImpl for ExMenuButton {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.parent_size_allocate(width, height, baseline);
        self.popover.present();
    }
}

impl BuildableImpl for ExMenuButton {}
