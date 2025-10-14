use std::{cell::Ref, ops::DerefMut};

use adw::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::property::PropertySet;
use gtk::glib::{self};
use log::warn;

use crate::systemd::UnitPropertyFetch;

pub const INTERFACE_NAME: &str = "Default";

glib::wrapper! {
    pub struct PropertyBrowseItem(ObjectSubclass<imp::PropertyBrowseItemImp>);
}

impl PropertyBrowseItem {
    pub fn new_interface(interface: String) -> Self {
        let this_object: Self = glib::Object::new();
        this_object.imp().interface.replace(interface);

        this_object
    }

    pub fn from(p: UnitPropertyFetch) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        p_imp.unit_property.replace(p.name);
        p_imp.signature.replace(Some(p.signature));
        p_imp.access.replace(Some(p.access));

        this_object
    }

    pub fn from_column(column: &gtk::ColumnViewColumn) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();

        p_imp.interface.replace(INTERFACE_NAME.to_owned());

        if let Some(property_name) = column.title() {
            p_imp.unit_property.replace(property_name.to_string());
        } else {
            warn!("Column with no title");
        };

        p_imp.column.replace(Some(column.clone()));

        this_object
    }

    pub fn from_parent(interface: PropertyBrowseItem, property: PropertyBrowseItem) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        p_imp.interface.replace(interface.interface());
        p_imp.unit_property.replace(property.unit_property());
        p_imp.signature.replace(property.signature());
        p_imp.access.replace(property.access());
        p_imp.column.replace(property.column());

        this_object
    }

    pub fn add_child(&self, child: PropertyBrowseItem) {
        //v.as_deref_mut().push(child);
        if let Some(v) = self.imp().children.borrow_mut().deref_mut() {
            v.push(child);
        } else {
            let v = vec![child];
            self.imp().children.set(Some(v));
        }
    }

    pub fn children(&self) -> Ref<'_, Option<Vec<PropertyBrowseItem>>> {
        self.imp().children.borrow()
    }
}

mod imp {
    use std::cell::RefCell;

    use crate::gtk::prelude::ObjectExt;
    use gtk::{glib, subclass::prelude::*};
    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::PropertyBrowseItem)]
    pub struct PropertyBrowseItemImp {
        #[property(get)]
        pub(super) interface: RefCell<String>,
        #[property(get)]
        pub(super) unit_property: RefCell<String>,
        #[property(get)]
        pub(super) signature: RefCell<Option<String>>,
        #[property(get)]
        pub(super) access: RefCell<Option<String>>,
        #[property(get)]
        pub(super) column: RefCell<Option<gtk::ColumnViewColumn>>,

        pub(super) children: RefCell<Option<Vec<super::PropertyBrowseItem>>>,
    }

    impl PropertyBrowseItemImp {}

    #[glib::object_subclass]
    impl ObjectSubclass for PropertyBrowseItemImp {
        const NAME: &'static str = "PropertyBrowseItem";
        type Type = super::PropertyBrowseItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PropertyBrowseItemImp {}
    impl PropertyBrowseItemImp {}
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use sourceview5::prelude::ToValue;

    #[test]
    fn pizza() {
        let s: Rc<str> = "testing".into();
        let asdf = s.to_value();
        println!("String {s} {:?}", asdf)
    }
}
