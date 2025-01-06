use gtk::{
    glib::{self},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
    TemplateChild,
};

use std::cell::{Cell, RefCell};

use log::{debug, warn};

use crate::systemd::{self, data::UnitInfo, enums::DependencyType};

/* const PANEL_EMPTY: &str = "empty";
const PANEL_JOURNAL: &str = "journal";
const PANEL_SPINNER: &str = "spinner";
 */
#[derive(Default, glib::Properties, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_dependencies_panel.ui")]
#[properties(wrapper_type = super::UnitDependenciesPanel)]
pub struct UnitDependenciesPanelImp {
    #[template_child]
    unit_dependencies_panel_stack: TemplateChild<gtk::Stack>,

    #[template_child]
    unit_dependencies_textview: TemplateChild<gtk::TextView>,

    #[property(get, set=Self::set_visible_on_page)]
    visible_on_page: Cell<bool>,

    #[property(get, set=Self::set_unit)]
    unit: RefCell<Option<UnitInfo>>,

    #[property(get, set)]
    dark: Cell<bool>,

    unit_dependencies_loaded: Cell<bool>,
}

#[gtk::template_callbacks]
impl UnitDependenciesPanelImp {
    fn set_visible_on_page(&self, value: bool) {
        debug!("set_visible_on_page val {value}");
        self.visible_on_page.set(value);

        if self.visible_on_page.get()
            && !self.unit_dependencies_loaded.get()
            && self.unit.borrow().is_some()
        {
            self.update_dependencies()
        }
    }

    fn set_unit(&self, unit: &UnitInfo) {
        let old_unit = self.unit.replace(Some(unit.clone()));
        if let Some(old_unit) = old_unit {
            if old_unit.primary() != unit.primary() {
                self.unit_dependencies_loaded.set(false)
            }
        }

        if self.visible_on_page.get() {
            self.update_dependencies()
        }
    }

    fn update_dependencies(&self) {
        let binding = self.unit.borrow();
        let Some(unit_ref) = binding.as_ref() else {
            warn!("No unit file");
            return;
        };

        let dep_type = DependencyType::Forward;
        let _results = systemd::fetch_unit_dependencies(unit_ref, dep_type);
    }
}
// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitDependenciesPanelImp {
    const NAME: &'static str = "UnitDependenciesPanel";
    type Type = super::UnitDependenciesPanel;
    type ParentType = gtk::Box;

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
impl ObjectImpl for UnitDependenciesPanelImp {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for UnitDependenciesPanelImp {}
impl BoxImpl for UnitDependenciesPanelImp {}
