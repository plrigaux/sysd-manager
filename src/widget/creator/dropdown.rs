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
    use std::cell::{Cell, OnceCell, RefCell};

    use adw::{prelude::ActionRowExt, subclass::prelude::*};
    use glib::{
        object::{Cast, CastNone, IsA},
        subclass::{object::ObjectImpl, types::ObjectSubclass},
    };
    use gtk::prelude::*;
    use tracing::{debug, error};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::SysDDropDown)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/dropdown.ui")]
    // #[properties(wrapper_type = super::SDDropdown)]
    pub struct SysDDropdownImp {
        #[template_child]
        arrow_down_image: TemplateChild<gtk::Image>,

        #[property(get, set)]
        popup_visible: Cell<bool>,

        filter_list_model: OnceCell<gtk::FilterListModel>,

        last_filter_string: RefCell<String>,

        custom_filter: OnceCell<gtk::CustomFilter>,

        search_entry: OnceCell<gtk::SearchEntry>,

        popover: OnceCell<gtk::Popover>,

        drop_list_view: OnceCell<gtk::ListView>,
    }

    #[gtk::template_callbacks]
    impl SysDDropdownImp {
        pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
            if let Some(fl) = self.filter_list_model.get() {
                fl.set_model(model);
            }
        }

        #[template_callback]
        fn search_entry_changed(&self, search_entry: &gtk::SearchEntry) {
            let text: glib::GString = search_entry.text();

            let mut last_filter = self.last_filter_string.borrow_mut();

            let text_is_empty = text.is_empty();
            if !text_is_empty {
                // self.toogle_button.set_active(true);
            }

            let change_type = if text_is_empty {
                gtk::FilterChange::LessStrict
            } else if text.len() > last_filter.len() && text.contains(last_filter.as_str()) {
                gtk::FilterChange::MoreStrict
            } else if text.len() < last_filter.len() && last_filter.contains(text.as_str()) {
                gtk::FilterChange::LessStrict
            } else {
                gtk::FilterChange::Different
            };

            debug!("Search text. Current \"{text}\" Prev \"{last_filter}\"");
            last_filter.replace_range(.., text.as_str());

            if let Some(custom_filter) = self.custom_filter.get() {
                custom_filter.changed(change_type);
            }
        }

        fn create_filter(&self) -> gtk::CustomFilter {
            let search_entry = self.search_entry().clone();

            gtk::CustomFilter::new(move |object| {
                let text_gs = search_entry.text();
                if text_gs.is_empty() {
                    return true;
                }

                let Some(list_item) = object.downcast_ref::<gtk::StringObject>() else {
                    error!("some wrong downcast_ref {object:?}");
                    return false;
                };

                let texts = text_gs.as_str();

                //if an upper case --> filter
                if text_gs.chars().any(|c| c.is_ascii_uppercase()) {
                    list_item.string().contains(texts)
                } else {
                    list_item.string().to_ascii_lowercase().contains(texts)
                }
            })
        }

        fn popover(&self) -> &gtk::Popover {
            let this = self.obj().clone();
            self.popover.get_or_init(|| {
                let pop = gtk::Popover::builder()
                    .css_classes(["menu"])
                    .autohide(false) //for not loosing the focus
                    // .autohide(true)
                    .has_arrow(true)
                    .height_request(300)
                    .width_request(200)
                    .position(gtk::PositionType::Bottom)
                    // .can_focus(false)
                    .build();

                pop.set_parent(&this.imp().arrow_down_image.get());

                let boxx = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .build();
                boxx.append(self.search_entry());
                boxx.append(self.drop_list_view());

                let scroll = gtk::ScrolledWindow::builder().child(&boxx).build();

                pop.set_child(Some(&scroll));
                pop
            })
        }

        fn drop_list_view(&self) -> &gtk::ListView {
            self.drop_list_view
                .get_or_init(|| gtk::ListView::builder().build())
        }

        fn search_entry(&self) -> &gtk::SearchEntry {
            self.search_entry
                .get_or_init(|| gtk::SearchEntry::builder().build())
        }

        fn set_popup_visible(&self, visible: bool) {
            if visible {
                // if let Some(single_selection) = self.single_selection.get() {
                //     single_selection.set_selected(gtk::INVALID_LIST_POSITION);
                // }
                self.popover().popup();
            } else {
                self.popover().popdown();
            }

            // Set the porperty indicator
            self.obj().set_popup_visible(visible);
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
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SysDDropdownImp {
        fn constructed(&self) {
            self.parent_constructed();

            let filter = self.create_filter();

            self.custom_filter
                .set(filter.clone())
                .expect("custom filter set once");

            let filter_list_model = gtk::FilterListModel::new(None::<gio::ListStore>, Some(filter));
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

            self.drop_list_view().set_model(Some(&selection_model));
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

            self.drop_list_view().set_factory(Some(&factory));

            // self.obj().connect_activated(|a| info!("asd"));

            let this = self.obj().clone();
            let gesture = gtk::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                let visible = this.popup_visible();
                this.imp().set_popup_visible(!visible);
            });

            self.obj().add_controller(gesture);

            let motion = gtk::EventControllerMotion::new();

            const CLASS: &str = "dimmed";
            let this = self.obj().clone();
            motion.connect_enter(move |_, _, _| {
                this.add_css_class(CLASS);
            });

            let this = self.obj().clone();
            motion.connect_leave(move |_| {
                this.remove_css_class(CLASS);
            });

            self.obj().add_controller(motion);
        }
    }

    impl WidgetImpl for SysDDropdownImp {}
    impl ListBoxRowImpl for SysDDropdownImp {}
    impl PreferencesRowImpl for SysDDropdownImp {}
    impl ActionRowImpl for SysDDropdownImp {}
}
