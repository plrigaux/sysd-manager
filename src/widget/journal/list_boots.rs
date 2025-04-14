use crate::widget::app_window::AppWindow;
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
    pub fn new(app_window: &AppWindow) -> Self {
        let obj: ListBootsWindow = glib::Object::new();
        let _ = obj.imp().app_window.set(app_window.clone());
        obj.imp().fill_store();
        obj
    }
}

mod imp {

    use std::{
        cell::{OnceCell, Ref},
        collections::HashMap,
        ops::DerefMut,
        rc::Rc,
    };

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

    use super::ListBootsWindow;
    use crate::{
        systemd::{self, journal::Boot},
        utils::th::{format_timestamp_relative_duration, get_since_time},
        widget::{app_window::AppWindow, preferences::data::PREFERENCES},
    };

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/list_boots.ui")]
    pub struct ListBootsWindowImp {
        #[template_child]
        list_store: TemplateChild<gio::ListStore>,

        #[template_child]
        boots_browser: TemplateChild<gtk::ColumnView>,

        #[template_child]
        stack: TemplateChild<adw::ViewStack>,

        pub app_window: OnceCell<AppWindow>,
    }

    //#[gtk::template_callbacks]
    impl ListBootsWindowImp {
        pub(super) fn fill_store(&self) {
            let stack = self.stack.clone();
            let list_store = self.list_store.clone();
            let app_window = self.app_window.get().unwrap().clone();

            glib::spawn_future_local(async move {
                stack.set_visible_child_name("spinner");
                list_store.remove_all();

                if app_window.imp().cached_list_boots().as_ref().is_none() {
                    let boots = gio::spawn_blocking(systemd::list_boots)
                        .await
                        .expect("Task needs to finish successfully.");

                    let boots = match boots {
                        Ok(boots) => {
                            let boots: Vec<Rc<Boot>> = boots.into_iter().map(Rc::new).collect();
                            boots
                        }
                        Err(error) => {
                            warn!("List boots Error {:?}", error);
                            return;
                        }
                    };

                    app_window.imp().update_list_boots(boots);
                } else {
                    //TODO find the last log

                    let last_time = gio::spawn_blocking(systemd::fetch_last_time)
                        .await
                        .expect("Task needs to finish successfully.");

                    let last_time = match last_time {
                        Ok(last_time) => last_time,
                        Err(error) => {
                            warn!("Fetch_last_time  Error {:?}", error);
                            return;
                        }
                    };

                    let mut binding = app_window.imp().cached_list_boots_mut();
                    if let Some(boots) = binding.deref_mut() {
                        if let Some(boot) = boots.pop() {
                            let new_boot = Boot {
                                boot_id: boot.boot_id.clone(),
                                last: last_time,
                                ..*boot.as_ref()
                            };

                            boots.push(Rc::new(new_boot));
                        }
                    }
                }

                let binding = app_window.imp().cached_list_boots();
                let Some(boots) = binding.as_ref() else {
                    warn!("Something wrong");
                    return;
                };

                for boot in boots.iter() {
                    let bx = BoxedAnyObject::new(boot.clone());
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

    fn setup(
        column_view_column_map: &HashMap<glib::GString, gtk::ColumnViewColumn>,
        key: &str,
    ) -> gtk::SignalListItemFactory {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Inscription::builder().css_classes(["mono"]).build();
            item.set_child(Some(&row));
        });

        column_view_column_map
            .get(key)
            .unwrap()
            .set_factory(Some(&factory));

        factory
    }

    macro_rules! bind {
        ($factory:expr, $body:expr) => {{
            $factory.connect_bind(move |_factory, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
                let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
                let boot: Ref<Rc<Boot>> = entry.borrow();

                ($body)(child, boot)
            });
        }};
    }

    fn set_up_factories(column_view_column_map: &HashMap<glib::GString, gtk::ColumnViewColumn>) {
        let col1factory = setup(column_view_column_map, "index");
        let col2factory = setup(column_view_column_map, "boot_id");
        let col3factory = setup(column_view_column_map, "firstlog");
        let col4factory = setup(column_view_column_map, "lastlog");
        let col5factory = setup(column_view_column_map, "duration");

        let bada = |child: gtk::Inscription, boot: Ref<Rc<Boot>>| {
            child.set_text(Some(&boot.index.to_string()))
        };
        bind!(col1factory, bada);
        let bada = |child: gtk::Inscription, boot: Ref<Rc<Boot>>| {
            child.set_text(Some(&boot.boot_id.to_string()))
        };
        bind!(col2factory, bada);

        let timestamp_style = PREFERENCES.timestamp_style();
        let bada = move |child: gtk::Inscription, boot: Ref<Rc<Boot>>| {
            let time = get_since_time(boot.first, timestamp_style);
            child.set_text(Some(&time));
        };
        bind!(col3factory, bada);

        let bada = move |child: gtk::Inscription, boot: Ref<Rc<Boot>>| {
            let time = get_since_time(boot.last, timestamp_style);
            child.set_text(Some(&time));
        };

        bind!(col4factory, bada);
        let bada = |child: gtk::Inscription, boot: Ref<Rc<Boot>>| {
            let duration = format_timestamp_relative_duration(boot.first, boot.last);
            child.set_text(Some(&duration));
        };
        bind!(col5factory, bada);
    }

    impl WidgetImpl for ListBootsWindowImp {}
    impl WindowImpl for ListBootsWindowImp {}
    impl AdwWindowImpl for ListBootsWindowImp {}
}
