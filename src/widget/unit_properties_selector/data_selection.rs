use adw::subclass::prelude::ObjectSubclassIsExt;

use gtk::glib::{self};
use log::info;

use crate::{
    systemd::enums::UnitType,
    widget::{
        unit_list::menus::create_col_menu,
        unit_properties_selector::{data_browser::PropertyBrowseItem, save::UnitColumn},
    },
};

//pub const INTERFACE_NAME: &str = "Basic Columns";

glib::wrapper! {
    pub struct UnitPropertySelection(ObjectSubclass<imp2::UnitPropertySelectionImpl>);
}

impl UnitPropertySelection {
    /*     pub fn new_interface(interface: String) -> Self {
           let this_object: Self = glib::Object::new();
           this_object.imp().interface.replace(interface);

           this_object
       }
    */
    pub fn from_browser(broswer_property: PropertyBrowseItem) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        let interface = broswer_property.interface();
        let unit_type = UnitType::from_intreface(&interface);
        let unit_property = broswer_property.unit_property();
        p_imp.prop_type.replace(Some(broswer_property.signature()));
        p_imp.access.replace(broswer_property.access());
        p_imp.unit_type.set(unit_type);

        let col = if let Some(col) = broswer_property.column() {
            info!("COL {:?} {:?}", col.id(), col.title());
            col
        } else {
            let id = format!("{}@{}", unit_type.as_str(), unit_property); //IMPORTANT keep this format
            let menu = create_col_menu(&id, true);
            let col = gtk::ColumnViewColumn::builder()
                .title(&unit_property)
                .id(id)
                .header_menu(&menu)
                .build();
            info!("New COL {:?} {:?}", col.id(), col.title());
            col
        };

        p_imp.unit_property.replace(unit_property);
        p_imp.column.replace(col);

        this_object
    }

    pub fn from_column_view_column(column: gtk::ColumnViewColumn) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();

        let property_name = column
            .title()
            .map(|t| t.to_string())
            .unwrap_or("Wrong_prop".to_string());
        p_imp.unit_property.replace(property_name);

        if let Some(id) = column.id() {
            Self::fill_from_id(p_imp, id.as_str());
        } else {
            p_imp.unit_type.set(UnitType::Unknown);
        }

        p_imp.column.replace(column);

        this_object
    }

    fn fill_from_id(p_imp: &imp2::UnitPropertySelectionImpl, id: &str) {
        if let Some((short_interface, prop)) = id.split_once('@') {
            p_imp.unit_property.replace(prop.to_string());
            let unit_type = UnitType::new(short_interface);
            p_imp.unit_type.set(unit_type);
        } else {
            p_imp.unit_type.set(UnitType::Unknown);
        }

        info!("UNIT TYPE FROM ID {} {:?}", id, p_imp.unit_type.get());
    }
    pub fn from_column_config(unit_column_config: UnitColumn) -> Self {
        let id = unit_column_config.id;
        //Self::fill_from_id(p_imp, &id);

        let column = gtk::ColumnViewColumn::builder()
            .id(&id)
            .fixed_width(unit_column_config.fixed_width)
            .expand(unit_column_config.expands)
            .resizable(unit_column_config.resizable)
            .visible(unit_column_config.visible)
            .build();

        let this_object = Self::from_column_view_column(column);

        let p_imp = this_object.imp();

        if let Some(title) = unit_column_config.title {
            p_imp.column.borrow().set_title(Some(&title));
            if !this_object.is_custom() {
                p_imp.unit_property.replace(title);
            }
        }

        p_imp.prop_type.replace(unit_column_config.prop_type);

        this_object
    }

    pub fn from_column(column_name: String) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();

        p_imp.unit_property.replace(column_name);

        this_object
    }
    /*
    pub fn from_parent(interface: UnitPropertySelection, property: UnitPropertySelection) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();

        p_imp.unit_property.replace(property.unit_property());
        p_imp.unit_type.replace(property.unit_type());
        p_imp.prop_type.replace(property.prop_type());
        p_imp.access.replace(property.access());

        this_object
    } */

    pub fn is_custom(&self) -> bool {
        self.imp().is_custom()
    }

    pub fn copy(&self) -> Self {
        let this_object: Self = glib::Object::new();
        self.copy_to(&this_object);

        this_object
    }

    pub fn copy_to(&self, to: &Self) {
        let p_imp = to.imp();

        p_imp.unit_property.replace(self.unit_property());
        p_imp.prop_type.replace(self.prop_type());
        p_imp.access.replace(self.access());
        p_imp.unit_type.set(self.unit_type());

        {
            let col = self.column();
            let cur_col = p_imp.column.borrow();

            Self::copy_col_to_col(&col, &cur_col);
        }
    }

    pub fn copy_col_to_col(from: &gtk::ColumnViewColumn, to: &gtk::ColumnViewColumn) {
        to.set_expand(from.expands());
        to.set_factory(from.factory().as_ref());
        to.set_fixed_width(from.fixed_width());
        to.set_header_menu(from.header_menu().as_ref());
        to.set_id(from.id().as_deref());
        to.set_resizable(from.is_resizable());
        to.set_sorter(from.sorter().as_ref());
        to.set_title(from.title().as_deref());
        to.set_visible(from.is_visible());
    }
}

mod imp2 {
    use std::cell::{Cell, RefCell};

    use glib::GString;
    use gtk::{glib, prelude::*, subclass::prelude::*};

    use crate::systemd::enums::UnitType;

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::UnitPropertySelection)]
    pub struct UnitPropertySelectionImpl {
        /*         #[property(get)]
        pub(super) interface: RefCell<String>, */
        #[property(get)]
        pub(super) unit_property: RefCell<String>,
        #[property(get)]
        pub(super) prop_type: RefCell<Option<String>>,
        #[property(get)]
        pub(super) access: RefCell<String>,
        #[property(name = "visible", get= Self::visible, set= Self::set_visible, type = bool)]
        #[property(name = "id", get= Self::id, set= Self::set_id, type = Option<GString>)]
        #[property(name = "title", get= Self::title, set= Self::set_title, type = Option<GString>)]
        #[property(name = "fixed-width", get= Self::fixed_width, set= Self::set_fixed_width, type = i32)]
        #[property(name = "resizable", get= Self::resizable, set= Self::set_resizable, type = bool)]
        #[property(name = "expands", get= Self::expands, set= Self::set_expand, type = bool)]
        #[property(get, set)]
        pub(super) column: RefCell<gtk::ColumnViewColumn>,

        #[property(name = "interface", get= Self::interface, type = String)]
        #[property(get, default)]
        pub(super) unit_type: Cell<UnitType>,
    }

    impl UnitPropertySelectionImpl {
        pub fn is_custom(&self) -> bool {
            !matches!(self.unit_type.get(), UnitType::Unknown)
        }

        fn interface(&self) -> String {
            self.unit_type.get().interface().to_string()
        }

        fn visible(&self) -> bool {
            self.column.borrow().is_visible()
        }

        fn set_visible(&self, visible: bool) {
            self.column.borrow().set_visible(visible)
        }

        fn id(&self) -> Option<GString> {
            self.column.borrow().id()
        }

        fn set_id(&self, id: Option<&str>) {
            self.column.borrow().set_id(id)
        }

        fn title(&self) -> Option<GString> {
            self.column.borrow().title()
        }

        fn set_title(&self, title: Option<&str>) {
            self.column.borrow().set_title(title)
        }

        fn fixed_width(&self) -> i32 {
            self.column.borrow().fixed_width()
        }

        fn set_fixed_width(&self, fixed_width: i32) {
            self.column.borrow().set_fixed_width(fixed_width);
        }

        fn resizable(&self) -> bool {
            self.column.borrow().is_resizable()
        }

        fn set_resizable(&self, resizable: bool) {
            self.column.borrow().set_resizable(resizable)
        }

        fn expands(&self) -> bool {
            self.column.borrow().expands()
        }

        fn set_expand(&self, expand: bool) {
            self.column.borrow().set_expand(expand)
        }
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
