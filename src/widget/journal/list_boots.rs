use crate::widget::app_window::AppWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib::{self};

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

    const WINDOW_HEIGHT: &str = "list-boots-window-height";
    const WINDOW_WIDTH: &str = "list-boots-window-width";
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
    use log::{debug, error, info, warn};

    use super::ListBootsWindow;
    use crate::{
        systemd::{self, BootFilter, data::UnitInfo, journal::Boot},
        systemd_gui::new_settings,
        utils::th::{format_timestamp_relative_duration, get_since_time},
        widget::{InterPanelMessage, app_window::AppWindow, preferences::data::PREFERENCES},
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

        #[template_child]
        list_boots_sort_list_model: TemplateChild<gtk::SortListModel>,

        pub app_window: OnceCell<AppWindow>,
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

            self.load_window_size();

            let map = self.generate_column_map();

            let list_boots_windows = self.obj();
            set_up_factories(&map, &list_boots_windows);
        }
    }

    //#[gtk::template_callbacks]
    impl ListBootsWindowImp {
        pub(super) fn fill_store(&self) {
            let stack = self.stack.clone();
            let list_store = self.list_store.clone();
            let app_window = self.app_window.get().unwrap().clone();
            let window = self.obj().clone();

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
                            warn!("List boots Error {error:?}");
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
                            warn!("Fetch_last_time  Error {error:?}");
                            return;
                        }
                    };

                    let mut binding = app_window.imp().cached_list_boots_mut();
                    if let Some(boots) = binding.deref_mut()
                        && let Some(boot) = boots.pop() {
                            let new_boot = Boot {
                                boot_id: boot.boot_id.clone(),
                                last: last_time,
                                ..*boot.as_ref()
                            };

                            boots.push(Rc::new(new_boot));
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

                window.imp().set_sorter();

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

                column_view_column.connect_fixed_width_notify(|column| {
                    println!("{:?} {}", column.id(), column.fixed_width())
                });

                if let Some(id) = id {
                    col_map.insert(id, column_view_column.clone());
                } else {
                    warn!("Column {col_idx} has no id.")
                }
            }
            col_map
        }

        pub fn save_window_context(&self) -> Result<(), glib::BoolError> {
            // Get the size of the window

            let obj = self.obj();
            let (width, height) = obj.default_size();

            // Set the window state in `settings`
            let settings = crate::systemd_gui::new_settings();

            settings.set_int(WINDOW_WIDTH, width)?;

            settings.set_int(WINDOW_HEIGHT, height)?;

            Ok(())
        }

        fn load_window_size(&self) {
            // Get the window state from `settings`
            let settings = new_settings();

            let mut width = settings.int(WINDOW_WIDTH);
            let mut height = settings.int(WINDOW_HEIGHT);

            let obj = self.obj();
            let (def_width, def_height) = obj.default_size();

            if width < 0 {
                width = def_width;
                if width < 0 {
                    width = 1280;
                }
            }

            if height < 0 {
                height = def_height;
                if height < 0 {
                    height = 720;
                }
            }

            // Set the size of the window
            obj.set_default_size(width, height);
        }

        fn set_sorter(&self) {
            let sorter = self.boots_browser.sorter();

            self.list_boots_sort_list_model.set_sorter(sorter.as_ref());

            let item_out = self
                .boots_browser
                .columns()
                .item(0)
                .expect("Expect item x to be not None");

            //Sort on first column
            let c1 = item_out
                .downcast_ref::<gtk::ColumnViewColumn>()
                .expect("item.downcast_ref::<gtk::ColumnViewColumn>()");

            self.boots_browser
                .sort_by_column(Some(c1), gtk::SortType::Descending);
        }

        fn selected_unit(&self) -> Option<UnitInfo> {
            let app_window = self.app_window.get()?;
            app_window.selected_unit()
        }
    }

    macro_rules! compare_boots {
        ($boot1:expr, $boot2:expr, $func:ident) => {{
            $boot1.$func().cmp(&$boot2.$func()).into()
        }};

        ($boot1:expr, $boot2:expr, $func:ident, $($funcx:ident),+) => {{

            let ordering = $boot1.$func().cmp(&$boot2.$func());
            if ordering != core::cmp::Ordering::Equal {
                return ordering.into();
            }

            compare_boots!($boot1, $boot2, $($funcx),+)
        }};
    }

    macro_rules! create_column_filter {
        ($($func:ident),+) => {{
            gtk::CustomSorter::new(move |obj1, obj2| {
                let boxed = obj1.downcast_ref::<BoxedAnyObject>().unwrap();
                let boot1: Ref<Rc<Boot>> = boxed.borrow();

                let boxed = obj2.downcast_ref::<BoxedAnyObject>().unwrap();
                let boot2: Ref<Rc<Boot>> = boxed.borrow();

                compare_boots!(boot2, boot1, $($func),+)
            })
        }};
    }

    macro_rules! column_view_column_set_sorter {
        ($column_view_column:expr, $($func:ident),+) => {{
            let sorter = create_column_filter!($($func),+);
            $column_view_column.set_sorter(Some(&sorter));
        }};
    }

    fn setup<'a>(
        column_view_column_map: &'a HashMap<glib::GString, gtk::ColumnViewColumn>,
        key: &str,
    ) -> (gtk::SignalListItemFactory, &'a gtk::ColumnViewColumn) {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Label::builder()
                .selectable(true)
                .xalign(0.0)
                //.css_classes(["mono"])
                .build();
            item.set_child(Some(&row));
        });

        let col = column_view_column_map.get(key).unwrap();
        col.set_factory(Some(&factory));

        (factory, col)
    }

    fn setup_action(
        column_view_column_map: &HashMap<glib::GString, gtk::ColumnViewColumn>,
        list_boots_windows: &ListBootsWindow,
    ) {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Button::builder()
                .icon_name("funnel-symbolic")
                .tooltip_text("Filter Journal Events")
                .css_classes(["suggested-action", "circular"])
                .margin_end(10)
                .build();
            item.set_child(Some(&row));
        });

        let col = column_view_column_map.get("action").unwrap();
        col.set_factory(Some(&factory));

        let list_boots_windows = list_boots_windows.clone();
        factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().and_downcast::<gtk::Button>().unwrap();
            let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
            let boot: Ref<Rc<Boot>> = entry.borrow();

            {
                let boot_id: String = boot.boot_id.clone();
                let list_boots_windows = list_boots_windows.clone();
                child.connect_clicked(move |_button| {
                    info!("boot {boot_id}");

                    let Some(app_window) = list_boots_windows.imp().app_window.get() else {
                        warn!("No app window");
                        return;
                    };

                    app_window.set_inter_message(&InterPanelMessage::JournalFilterBoot(
                        BootFilter::Id(boot_id.clone()),
                    ));
                });
            }

            if let Some(_unit) = list_boots_windows.imp().selected_unit() {
                child.set_sensitive(true);
            } else {
                child.set_sensitive(false);
            }
        });
    }

    macro_rules! bind {
        ($factory:expr, $body:expr) => {{
            $factory.connect_bind(move |_factory, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let child = item.child().and_downcast::<gtk::Label>().unwrap();
                let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
                let boot: Ref<Rc<Boot>> = entry.borrow();

                ($body)(child, boot)
            });
        }};
    }

    fn set_up_factories(
        column_view_column_map: &HashMap<glib::GString, gtk::ColumnViewColumn>,
        list_boots_windows: &ListBootsWindow,
    ) {
        let (col1factory, col1) = setup(column_view_column_map, "pos_offset");
        column_view_column_set_sorter!(col1, index);
        let (col1bfactory, col1b) = setup(column_view_column_map, "neg_offset");
        column_view_column_set_sorter!(col1b, neg_offset);
        let (col2factory, _) = setup(column_view_column_map, "boot_id");
        let (col3factory, _) = setup(column_view_column_map, "firstlog");
        let (col4factory, _) = setup(column_view_column_map, "lastlog");
        let (col5factory, col5) = setup(column_view_column_map, "duration");
        setup_action(column_view_column_map, list_boots_windows);
        column_view_column_set_sorter!(col5, duration);

        let bada = |child: gtk::Label, boot: Ref<Rc<Boot>>| child.set_text(&boot.index.to_string());
        bind!(col1factory, bada);

        let bada =
            |child: gtk::Label, boot: Ref<Rc<Boot>>| child.set_text(&boot.neg_offset().to_string());
        bind!(col1bfactory, bada);
        let bada =
            |child: gtk::Label, boot: Ref<Rc<Boot>>| child.set_text(&boot.boot_id.to_string());

        bind!(col2factory, bada);

        let timestamp_style = PREFERENCES.timestamp_style();
        let bada = move |child: gtk::Label, boot: Ref<Rc<Boot>>| {
            let time = get_since_time(boot.first, timestamp_style);
            child.set_text(&time);
        };
        bind!(col3factory, bada);

        let bada = move |child: gtk::Label, boot: Ref<Rc<Boot>>| {
            let time = get_since_time(boot.last, timestamp_style);
            child.set_text(&time);
        };

        bind!(col4factory, bada);
        let bada = |child: gtk::Label, boot: Ref<Rc<Boot>>| {
            let duration = format_timestamp_relative_duration(boot.first, boot.last);
            child.set_text(&duration);
        };
        bind!(col5factory, bada);
    }

    impl WidgetImpl for ListBootsWindowImp {}

    impl WindowImpl for ListBootsWindowImp {
        // Save window state right before the window will be closed
        fn close_request(&self) -> glib::Propagation {
            // Save window size
            debug!("Close window");
            if let Err(_err) = self.save_window_context() {
                error!("Failed to save window state");
            }

            self.parent_close_request();
            // Allow to invoke other event handlers
            glib::Propagation::Proceed
        }
    }

    impl AdwWindowImpl for ListBootsWindowImp {}
}
