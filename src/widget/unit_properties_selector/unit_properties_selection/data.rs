use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib::{self};

use crate::widget::unit_properties_selector::data::PropertiesSelectorObject;

glib::wrapper! {
    pub struct UnitPropertySelection(ObjectSubclass<imp::UnitPropertySelectionImpl>);
}

impl UnitPropertySelection {
    pub fn new_interface(interface: String) -> Self {
        let this_object: Self = glib::Object::new();
        this_object.imp().interface.replace(interface);

        this_object
    }

    pub fn from_po(p: PropertiesSelectorObject) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        p_imp.interface.replace(p.interface());
        p_imp.unit_property.replace(p.unit_property());
        p_imp.signature.replace(p.signature());
        p_imp.access.replace(p.access());

        this_object
    }

    pub fn from_column(column_name: String) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();

        p_imp.unit_property.replace(column_name);

        this_object
    }

    pub fn from_parent(interface: UnitPropertySelection, property: UnitPropertySelection) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        p_imp.interface.replace(interface.interface());
        p_imp.unit_property.replace(property.unit_property());
        p_imp.signature.replace(property.signature());
        p_imp.access.replace(property.access());

        this_object
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::UnitPropertySelection)]
    pub struct UnitPropertySelectionImpl {
        #[property(get)]
        pub(super) interface: RefCell<String>,
        #[property(get)]
        pub(super) unit_property: RefCell<String>,
        #[property(get)]
        pub(super) signature: RefCell<String>,
        #[property(get)]
        pub(super) access: RefCell<String>,
        #[property(get)]
        pub(super) column: RefCell<gtk::ColumnViewColumn>,
        #[property(get)]
        pub(super) hidden: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnitPropertySelectionImpl {
        const NAME: &'static str = "UnitPropertySelection";
        type Type = super::UnitPropertySelection;
    }

    #[glib::derived_properties]
    impl ObjectImpl for UnitPropertySelectionImpl {}
    impl UnitPropertySelectionImpl {}
}
