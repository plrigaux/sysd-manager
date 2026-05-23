//test

use glib::{object::IsA, subclass::types::ObjectSubclassIsExt};

glib::wrapper! {
    pub struct SysDDropDown(ObjectSubclass<imp::SysDDropdownImp>)
    @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible,gtk::Actionable,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl SysDDropDown {
    pub fn new() -> Self {
        let obj: SysDDropDown = glib::Object::new();
        obj
    }

    pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
        self.imp().set_model(model)
    }
}

impl Default for SysDDropDown {
    fn default() -> Self {
        SysDDropDown::new()
    }
}

mod imp {
    use std::cell::OnceCell;

    use adw::{prelude::ActionRowExt, subclass::prelude::*};
    use glib::{
        object::{Cast, CastNone, IsA},
        subclass::{object::ObjectImpl, types::ObjectSubclass},
    };
    use gtk::prelude::ListItemExt;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/dropdown.ui")]
    // #[properties(wrapper_type = super::SDDropdown)]
    pub struct SysDDropdownImp {
        #[template_child]
        drop_list_view: TemplateChild<gtk::ListView>,

        filter_list_model: OnceCell<gtk::FilterListModel>,
    }

    impl SysDDropdownImp {
        pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
            if let Some(fl) = self.filter_list_model.get() {
                fl.set_model(model);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SysDDropdownImp {
        const NAME: &'static str = "SysDDropDown";
        type Type = super::SysDDropDown;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            // klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // #[glib::derived_properties]
    impl ObjectImpl for SysDDropdownImp {
        fn constructed(&self) {
            self.parent_constructed();

            // let list_store = gio::ListStore::new::<gtk::StringObject>();
            // for i in 0..50 {
            //     vec.push(i.to_string());
            //     let item = gtk::StringObject::new(&i.to_string());
            //     list_store.append(&item);
            // }

            //let selection_model = gtk::NoSelection::new(Some(store.clone()));

            // gtk::Filter:: c

            let filter_list_model =
                gtk::FilterListModel::new(None::<gio::ListStore>, None::<gtk::Filter>);
            let selection_model = gtk::SingleSelection::builder()
                .can_unselect(true)
                .autoselect(false)
                .model(&filter_list_model)
                .build();

            let _ = self.filter_list_model.set(filter_list_model);
            let action_row = self.obj().clone();
            selection_model.connect_selected_notify(move |a| {
                // println!("xx");
                if let Some(s) = a.selected_item().and_downcast_ref::<gtk::StringObject>() {
                    // println!("{}", s.string())

                    action_row.set_subtitle(&s.string());
                }
            });

            self.drop_list_view.set_model(Some(&selection_model));
            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(move |_factory, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let row = gtk::Label::builder().xalign(0.0).build();
                item.set_child(Some(&row));
            });

            factory.connect_bind(move |_factory, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let data = item.item().and_downcast::<gtk::StringObject>().unwrap();

                let child = item.child().and_downcast::<gtk::Label>().unwrap();

                child.set_label(&data.string());
            });

            self.drop_list_view.set_factory(Some(&factory));
        }
    }

    impl WidgetImpl for SysDDropdownImp {}
    impl ListBoxRowImpl for SysDDropdownImp {}
    impl PreferencesRowImpl for SysDDropdownImp {}
    impl ActionRowImpl for SysDDropdownImp {}
}
