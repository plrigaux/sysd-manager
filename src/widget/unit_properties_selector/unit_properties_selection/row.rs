use gtk::{
    ListItem,
    glib::{self},
    subclass::prelude::*,
};

use crate::widget::unit_properties_selector::{
    data2::UnitPropertySelection, unit_properties_selection::UnitPropertiesSelection,
};

glib::wrapper! {
    pub struct UnitPropertiesSelectionRow(ObjectSubclass<imp::UnitPropertiesSelectionRowImp>)
    @extends gtk::Grid, gtk::Widget,
    @implements gtk::Actionable, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitPropertiesSelectionRow {
    pub fn new(prop_selection: UnitPropertiesSelection) -> Self {
        let obj: UnitPropertiesSelectionRow = glib::Object::new();

        obj.imp().prop_selection.replace(Some(prop_selection));

        obj
    }

    pub fn set_data_selection(&self, prop_seletion: &UnitPropertySelection, list_item: &ListItem) {
        self.imp().set_data_selection(prop_seletion, list_item)
    }

    pub fn unbind(&self) {
        self.imp().unbind()
    }
}

impl Default for UnitPropertiesSelectionRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use std::cell::RefCell;

    use crate::widget::unit_properties_selector::{
        data2::UnitPropertySelection, unit_properties_selection::UnitPropertiesSelection,
    };

    use super::UnitPropertiesSelectionRow;
    use gio::glib::Binding;
    use glib::GString;
    use gtk::{
        glib::{self},
        prelude::*,
        subclass::prelude::*,
    };
    use log::warn;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_properties_selection_row.ui")]
    pub struct UnitPropertiesSelectionRowImp {
        #[template_child]
        interface: TemplateChild<gtk::Label>,
        #[template_child]
        property_label: TemplateChild<gtk::Label>,
        #[template_child]
        hidden_ckeck: TemplateChild<gtk::CheckButton>,
        #[template_child]
        title_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        width_spin: TemplateChild<gtk::SpinButton>,
        #[template_child]
        resizable_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        expand_switch: TemplateChild<gtk::Switch>,

        pub prop_selection: RefCell<Option<UnitPropertiesSelection>>,

        list_item: RefCell<Option<gtk::ListItem>>,

        binds: RefCell<Vec<Binding>>,
    }

    #[gtk::template_callbacks]
    impl UnitPropertiesSelectionRowImp {
        pub(super) fn set_data_selection(
            &self,
            prop_selection: &UnitPropertySelection,
            list_item: &gtk::ListItem,
        ) {
            self.list_item.replace(Some(list_item.clone()));

            let interface = prop_selection.interface();

            if let Some(token) = interface.split('.').next_back() {
                self.interface.set_label(token);
            } else {
                self.interface.set_label(&interface);
            }

            self.property_label
                .set_label(&prop_selection.unit_property());

            self.hidden_ckeck.set_active(prop_selection.visible());
            let bind = self
                .hidden_ckeck
                .bind_property("active", prop_selection, "visible")
                .build();

            self.binds.borrow_mut().push(bind);

            let title = prop_selection.title().unwrap_or_default();
            self.title_entry.buffer().set_text(title);
            let bind = self
                .title_entry
                .buffer()
                .bind_property("text", prop_selection, "title")
                .transform_to(|_b, s: GString| Some(s))
                .build();

            self.binds.borrow_mut().push(bind);

            self.width_spin
                .set_value(prop_selection.fixed_width() as f64);
            let bind = self
                .width_spin
                .bind_property("value", prop_selection, "fixed-width")
                .transform_to(|_b, v: f64| Some(v as i32))
                .build();

            self.binds.borrow_mut().push(bind);

            self.resizable_switch.set_active(prop_selection.resizable());
            let bind = self
                .resizable_switch
                .bind_property("active", prop_selection, "resizable")
                .build();

            self.binds.borrow_mut().push(bind);

            self.expand_switch.set_active(prop_selection.expands());
            let bind = self
                .expand_switch
                .bind_property("active", prop_selection, "expands")
                .build();

            self.binds.borrow_mut().push(bind);
        }

        pub(super) fn unbind(&self) {
            {
                for b in self.binds.borrow().iter() {
                    b.unbind()
                }
            }

            self.binds.borrow_mut().clear();

            self.list_item.replace(None);
        }
        #[template_callback]
        fn move_up_clicked(&self, _b: &gtk::Button) {
            let Some(ref list_item) = *self.list_item.borrow() else {
                return;
            };

            let pos = list_item.position();
            if pos == 0 {
                return;
            }

            if let Some(list_store) = self
                .prop_selection
                .borrow()
                .as_ref()
                .and_then(|prop_selection| prop_selection.list_store())
                && let Some(ref prop_seletion) = list_store.item(pos)
            {
                list_store.remove(pos);
                list_store.insert(pos - 1, prop_seletion);
            } else {
                warn!("list_store of data None");
            };
        }

        #[template_callback]
        fn move_down_clicked(&self, _b: &gtk::Button) {
            let Some(ref list_item) = *self.list_item.borrow() else {
                return;
            };

            if let Some(prop_selection) = self.prop_selection.borrow().as_ref() {
                let pos = list_item.position();
                prop_selection.move_down(pos);
            }
        }

        #[template_callback]
        fn delete_clicked(&self, _b: &gtk::Button) {
            let Some(ref list_item) = *self.list_item.borrow() else {
                return;
            };

            if let Some(prop_selection) = self.prop_selection.borrow().as_ref() {
                let pos = list_item.position();
                prop_selection.delete(pos);
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for UnitPropertiesSelectionRowImp {
        const NAME: &'static str = "UnitPropertiesSelectorRow";
        type Type = UnitPropertiesSelectionRow;
        type ParentType = gtk::Grid;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnitPropertiesSelectionRowImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for UnitPropertiesSelectionRowImp {}
    impl GridImpl for UnitPropertiesSelectionRowImp {}
}
