use std::{cell::RefCell, collections::HashMap};

use gtk::{glib, prelude::*, subclass::prelude::*};

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

    pub(super) check_boxes : RefCell<HashMap<String, gtk::CheckButton>>
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
    }

    #[template_callback(name = "clear_filter_selection")]
    fn clear_filter_selection(&self, _button : &gtk::Button) {
        let  map = self.check_boxes.borrow();

        for chec_button in map.values().into_iter() {
            chec_button.set_active(false);
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

    fn constructed(&self) {
       
    }
}

impl WidgetImpl for ExMenuButton {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.parent_size_allocate(width, height, baseline);
        self.popover.present();
    }
}

impl BuildableImpl for ExMenuButton {}
