use std::{cell::Ref, ops::DerefMut};

use adw::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::property::PropertySet;
use gtk::glib::{self};

use crate::systemd::{UnitPropertyFetch, enums::UnitType};

pub const INTERFACE_NAME: &str = "Basic Columns";

glib::wrapper! {
    pub struct PropertiesSelectorObject(ObjectSubclass<imp::PropertiesSelectorOpjectImpl>);
}

impl PropertiesSelectorObject {
    pub fn new_interface(interface: String) -> Self {
        let this_object: Self = glib::Object::new();
        this_object.imp().interface.replace(interface);

        this_object
    }

    pub fn from(p: UnitPropertyFetch) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        p_imp.unit_property.replace(p.name);
        p_imp.signature.replace(p.signature);
        p_imp.access.replace(p.access);

        this_object
    }

    pub fn from_column(column_name: String) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();

        p_imp.unit_property.replace(column_name);

        this_object
    }

    pub fn from_parent(
        interface: PropertiesSelectorObject,
        property: PropertiesSelectorObject,
    ) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        p_imp.interface.replace(interface.interface());
        p_imp.unit_property.replace(property.unit_property());
        p_imp.signature.replace(property.signature());
        p_imp.access.replace(property.access());

        this_object
    }

    pub fn add_child(&self, child: PropertiesSelectorObject) {
        //v.as_deref_mut().push(child);
        if let Some(v) = self.imp().children.borrow_mut().deref_mut() {
            v.push(child);
        } else {
            let v = vec![child];
            self.imp().children.set(Some(v));
        }
    }

    pub fn children(&self) -> Ref<'_, Option<Vec<PropertiesSelectorObject>>> {
        self.imp().children.borrow()
    }
}

glib::wrapper! {
    pub struct UnitPropertySelection(ObjectSubclass<imp2::UnitPropertySelectionImpl>);
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
        let interface = p.interface();
        let unit_type = UnitType::from_intreface(&interface);
        p_imp.interface.replace(interface);
        p_imp.unit_property.replace(p.unit_property());
        p_imp.signature.replace(p.signature());
        p_imp.access.replace(p.access());
        p_imp.unit_type.set(unit_type);

        this_object
    }

    pub fn from_base_column(property_name: String, column: gtk::ColumnViewColumn) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        p_imp.interface.replace(INTERFACE_NAME.to_string());
        p_imp.unit_property.replace(property_name);
        //p_imp.signature.replace(p.signature());
        //p_imp.access.replace(p.access());

        p_imp.unit_type.set(UnitType::Unknown);

        p_imp.column.replace(column);

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
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::PropertiesSelectorObject)]
    pub struct PropertiesSelectorOpjectImpl {
        #[property(get)]
        pub(super) interface: RefCell<String>,
        #[property(get)]
        pub(super) unit_property: RefCell<String>,
        #[property(get)]
        pub(super) signature: RefCell<String>,
        #[property(get)]
        pub(super) access: RefCell<String>,

        pub(super) children: RefCell<Option<Vec<super::PropertiesSelectorObject>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PropertiesSelectorOpjectImpl {
        const NAME: &'static str = "PropertiesSelectorObject";
        type Type = super::PropertiesSelectorObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PropertiesSelectorOpjectImpl {}
    impl PropertiesSelectorOpjectImpl {}
}

mod imp2 {
    use std::cell::{Cell, RefCell};

    use gtk::{glib, prelude::*, subclass::prelude::*};

    use crate::systemd::enums::UnitType;

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
        #[property(get, default_value = false)]
        pub(super) hidden: Cell<bool>,
        #[property(get, default)]
        pub(super) unit_type: Cell<UnitType>,
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
