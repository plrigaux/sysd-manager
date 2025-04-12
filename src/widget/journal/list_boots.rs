use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib::{self};

// ANCHOR: mod
glib::wrapper! {

    pub struct ListBootsWindow(ObjectSubclass<imp::ListBootsWindowImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ListBootsWindow {
    pub fn new() -> Self {
        let obj: ListBootsWindow = glib::Object::new();
        obj.imp().fill_store();
        obj
    }
}

impl Default for ListBootsWindow {
    fn default() -> Self {
        ListBootsWindow::new()
    }
}

mod imp {

    use std::{cell::Ref, collections::HashMap};

    use adw::subclass::window::AdwWindowImpl;
    use gio::{glib::BoxedAnyObject, prelude::ListModelExt};
    use gtk::{
        glib::{self},
        prelude::*,
        subclass::{
            prelude::*,
            widget::{CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetImpl},
        },
    };
    use log::warn;

    use crate::{
        systemd::{self, journal::Boot},
        utils::th::{TimestampStyle, format_timestamp_relative_duration, get_since_time},
    };

    use super::ListBootsWindow;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/list_boots.ui")]
    pub struct ListBootsWindowImp {
        #[template_child]
        list_store: TemplateChild<gio::ListStore>,

        #[template_child]
        boots_browser: TemplateChild<gtk::ColumnView>,

        #[template_child]
        stack: TemplateChild<adw::ViewStack>,
    }

    //#[gtk::template_callbacks]
    impl ListBootsWindowImp {
        pub(super) fn fill_store(&self) {
            let stack = self.stack.clone();
            let list_store = self.list_store.clone();

            glib::spawn_future_local(async move {
                stack.set_visible_child_name("spinner");
                list_store.remove_all();

                let boots = gio::spawn_blocking(move || match systemd::list_boots() {
                    Ok(boots) => Ok(boots),
                    Err(error) => {
                        warn!("List boots Error {:?}", error);
                        Err(error)
                    }
                })
                .await
                .expect("Task needs to finish successfully.");

                let Ok(boots) = boots else {
                    return;
                };

                for boot in boots {
                    let bx = BoxedAnyObject::new(boot);
                    list_store.append(&bx);
                }

                stack.set_visible_child_name("list_boots");
            });
        }

        fn generate_column_map(&self) -> HashMap<glib::GString, gtk::ColumnViewColumn> {
            let list_model: gio::ListModel = self.boots_browser.columns();

            let mut col_map = HashMap::new();
            for col_idx in 0..list_model.n_items() {
                let item_out = list_model
                    .item(col_idx)
                    .expect("Expect item x to be not None");

                let column_view_column = item_out
                    .downcast_ref::<gtk::ColumnViewColumn>()
                    .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

                let id = column_view_column.id();

                if let Some(id) = id {
                    col_map.insert(id, column_view_column.clone());
                } else {
                    warn!("Column {col_idx} has no id.")
                }
            }
            col_map
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ListBootsWindowImp {
        const NAME: &'static str = "ListBoots";
        type Type = ListBootsWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            //klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ListBootsWindowImp {
        fn constructed(&self) {
            self.parent_constructed();

            let map = self.generate_column_map();

            set_up_factories(&map);
        }
    }

    fn set_up_factories(column_view_column_map: &HashMap<glib::GString, gtk::ColumnViewColumn>) {
        let col1factory = gtk::SignalListItemFactory::new();
        let col2factory = gtk::SignalListItemFactory::new();
        let col3factory = gtk::SignalListItemFactory::new();
        let col4factory = gtk::SignalListItemFactory::new();
        let col5factory = gtk::SignalListItemFactory::new();

        col1factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Inscription::default();
            item.set_child(Some(&row));
        });

        col2factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Inscription::default();
            item.set_child(Some(&row));
        });

        col3factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Inscription::default();
            item.set_child(Some(&row));
        });

        col4factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Inscription::default();
            item.set_child(Some(&row));
        });

        col5factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Inscription::default();
            item.set_child(Some(&row));
        });

        col1factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let boot: Ref<Boot> = entry.borrow();

            child.set_text(Some(&boot.index.to_string()));
        });

        col2factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let boot: Ref<Boot> = entry.borrow();

            child.set_text(Some(&boot.boot_id));
        });

        col3factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let boot: Ref<Boot> = entry.borrow();

            let time = get_since_time(boot.first, TimestampStyle::Pretty);
            child.set_text(Some(&time));
        });

        col4factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let boot: Ref<Boot> = entry.borrow();

            let time = get_since_time(boot.last, TimestampStyle::Pretty);
            child.set_text(Some(&time));
        });

        col5factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let boot: Ref<Boot> = entry.borrow();

            let duration = format_timestamp_relative_duration(boot.first, boot.last);
            child.set_text(Some(&duration));
        });

        column_view_column_map
            .get("index")
            .unwrap()
            .set_factory(Some(&col1factory));
        column_view_column_map
            .get("boot_id")
            .unwrap()
            .set_factory(Some(&col2factory));
        column_view_column_map
            .get("firstlog")
            .unwrap()
            .set_factory(Some(&col3factory));
        column_view_column_map
            .get("lastlog")
            .unwrap()
            .set_factory(Some(&col4factory));
        column_view_column_map
            .get("duration")
            .unwrap()
            .set_factory(Some(&col5factory));
    }

    impl WidgetImpl for ListBootsWindowImp {}
    impl WindowImpl for ListBootsWindowImp {}
    impl AdwWindowImpl for ListBootsWindowImp {}
}
