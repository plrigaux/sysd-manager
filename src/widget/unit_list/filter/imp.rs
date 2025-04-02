use std::cell::RefCell;

use adw::{prelude::*, subclass::window::AdwWindowImpl};
use gtk::{
    glib::{self, property::PropertySet},
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};
use strum::IntoEnumIterator;

use log::{info, warn};

use crate::widget::preferences::data::UNIT_LIST_COLUMNS;

use super::UnitListFilterWindow;

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_list_filter.ui")]
#[properties(wrapper_type = super::UnitListFilterWindow)]
pub struct UnitListFilterWindowImp {
    #[template_child]
    filter_stack: TemplateChild<gtk::Stack>,

    #[property(get, set, nullable, default = None)]
    selected: RefCell<Option<String>>,
}

#[gtk::template_callbacks]
impl UnitListFilterWindowImp {
    /*     #[template_callback]
    fn notify_visible_child_cb(&self, _stack: Param) {
        println!("notify_visible_child_cb");
    } */
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitListFilterWindowImp {
    const NAME: &'static str = "UNIT_LIST_FILTER";
    type Type = UnitListFilterWindow;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for UnitListFilterWindowImp {
    fn constructed(&self) {
        self.parent_constructed();

        for (name, key, _) in UNIT_LIST_COLUMNS {
            self.filter_stack
                .add_titled(&gtk::Label::new(Some(name)), Some(key), name);
        }

        self.obj()
            .bind_property::<gtk::Stack>(
                "selected",
                self.filter_stack.as_ref(),
                "visible-child-name",
            )
            .bidirectional()
            .build();
    }
}

impl WidgetImpl for UnitListFilterWindowImp {}
impl WindowImpl for UnitListFilterWindowImp {}
impl AdwWindowImpl for UnitListFilterWindowImp {}
