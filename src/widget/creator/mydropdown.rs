use glib::{object::IsA, subclass::types::ObjectSubclassIsExt};
use gtk::glib;

glib::wrapper! {
    pub struct MyDropDown(ObjectSubclass<imp::MyDropdownImp>)
    @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible,gtk::Actionable,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl MyDropDown {
    pub fn new() -> Self {
        let obj: MyDropDown = glib::Object::new();
        obj
    }

    pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
        self.imp().set_model(model)
    }
}

impl Default for MyDropDown {
    fn default() -> Self {
        MyDropDown::new()
    }
}

glib::wrapper! {
    pub struct DropData(ObjectSubclass<imp_data::DropDataImp>);
}

impl Default for DropData {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DropData {
    pub fn new(string: glib::GString) -> Self {
        glib::Object::builder().property("string", string).build()
    }
}

mod imp_data {

    use adw::subclass::prelude::*;
    use glib::object::ObjectExt;
    use std::cell::{Cell, OnceCell, RefCell};

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::DropData)]
    pub struct DropDataImp {
        #[property(get, set)]
        pub string: OnceCell<String>,
        #[property(get, set)]
        pub selected: Cell<bool>,

        pub binding: RefCell<Option<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DropDataImp {
        const NAME: &'static str = "DropData";
        type Type = super::DropData;
        type ParentType = glib::Object;
        fn new() -> Self {
            Default::default()
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DropDataImp {}
}

mod imp {
    use crate::upgrade;
    use adw::{prelude::ActionRowExt, subclass::prelude::*};
    use gtk::{ffi::GTK_INVALID_LIST_POSITION, gio::prelude::*, prelude::*};
    use std::cell::{Cell, OnceCell, RefCell};
    use tracing::{debug, error, info};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/mydropdown.ui")]
    #[properties(wrapper_type = super::MyDropDown)]
    pub struct MyDropdownImp {
        #[template_child]
        arrow_box: TemplateChild<gtk::Box>,

        #[template_child]
        popover: TemplateChild<gtk::Popover>,

        #[template_child]
        list: TemplateChild<gtk::ListView>,

        #[template_child]
        search_entry: TemplateChild<gtk::SearchEntry>,

        #[property(get=Self::enable_search, set=Self::set_enable_search, name = "enable-search",type=bool) ]
        // pub enable_search: Cell<bool>,
        filter_list_model: OnceCell<gtk::FilterListModel>,

        popup_selection_model: OnceCell<gtk::SingleSelection>,

        last_filter_string: RefCell<String>,

        custom_filter: OnceCell<gtk::CustomFilter>,

        factory: RefCell<Option<gtk::ListItemFactory>>,

        old_selected: Cell<u32>,

        selected: RefCell<Option<super::DropData>>,
    }

    #[gtk::template_callbacks]
    impl MyDropdownImp {
        fn popup_selection_model(&self) -> &gtk::SingleSelection {
            if let Some(model) = self.popup_selection_model.get() {
                model
            } else {
                let filter_model = self.filter_list_model();
                let single_selection = gtk::SingleSelection::builder()
                    .model(filter_model)
                    .autoselect(true)
                    .can_unselect(true)
                    .build();

                // single_selection.connect_selected_notify(|_| info!("p sel"));
                let _ = self.popup_selection_model.set(single_selection);
                self.popup_selection_model()
            }
        }

        fn filter_list_model(&self) -> &gtk::FilterListModel {
            if let Some(model) = self.filter_list_model.get() {
                model
            } else {
                let filter = self.create_filter();
                let filter_list_model = gtk::FilterListModel::builder().filter(filter).build();
                let _ = self.filter_list_model.set(filter_list_model);
                self.filter_list_model()
            }
        }

        pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
            if let Some(model) = model {
                let list = gio::ListStore::new::<super::DropData>();
                for i in 0..model.n_items() {
                    if let Some(x) = model.item(i).and_downcast::<gtk::StringObject>() {
                        list.append(&super::DropData::new(x.string()));
                    }
                }

                self.filter_list_model().set_model(Some(&list));
            } else {
                self.filter_list_model().set_model(model);
            }
            self.popup_selection_model()
                .set_model(Some(self.filter_list_model()));
            self.list.set_model(Some(self.popup_selection_model()));
        }

        #[template_callback]
        fn search_changed_cb(&self, search_entry: &gtk::SearchEntry) {
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

        #[template_callback]
        fn search_stop_cb(&self, _search_entry: &gtk::SearchEntry) {
            info!("search stop");
        }

        fn create_filter(&self) -> &gtk::CustomFilter {
            if let Some(filter) = self.custom_filter.get() {
                filter
            } else {
                let search_entry = self.search_entry.clone();

                let filter = gtk::CustomFilter::new(move |object| {
                    let text_gs = search_entry.text();
                    if text_gs.is_empty() {
                        return true;
                    }

                    let Some(list_item) = object.downcast_ref::<super::DropData>() else {
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
                });
                let _ = self.custom_filter.set(filter);
                self.custom_filter.get().unwrap()
            }
        }

        #[template_callback]
        fn notify_popover_visible_cb(&self, visible: glib::ParamSpec, popover: gtk::Popover) {
            info!(
                "pop up visible {:?} {}",
                visible.value_type(),
                visible.name() // visible.values()
            );

            const OPEN_GROUP_CSS: &str = "has-open-group";
            if popover.is_visible() {
                self.obj().add_css_class(OPEN_GROUP_CSS);
            } else {
                self.obj().remove_css_class(OPEN_GROUP_CSS);
            }
        }

        fn create_default_factory(&self) -> gtk::SignalListItemFactory {
            let factory = gtk::SignalListItemFactory::new();
            // let this = self.downgrade();
            factory.connect_setup(move |_factory, item| {
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let row_label = gtk::Label::builder().xalign(0.0).width_chars(1).build();

                let item_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .build();

                let icon = gtk::Image::builder()
                    .accessible_role(gtk::AccessibleRole::Presentation)
                    .icon_name("object-select-symbolic")
                    .build();
                item_box.append(&icon);
                item_box.append(&row_label);

                list_item.set_child(Some(&item_box));
            });

            let this = self.downgrade();
            factory.connect_bind(move |_factory, item| {
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let data = list_item.item().and_downcast::<super::DropData>().unwrap();

                let item_box = list_item.child().and_downcast::<gtk::Box>().unwrap();
                if let Some(label) = item_box.last_child().and_downcast_ref::<gtk::Label>() {
                    label.set_label(&data.string());
                };

                if let Some(image) = item_box.first_child().and_downcast::<gtk::Image>() {
                    let binding = data
                        .bind_property("selected", &image, "opacity")
                        .transform_to(|_, selected: bool| {
                            let opacity = if selected { 1.0 } else { 0.0 };
                            Some(opacity)
                        })
                        .build();

                    if let Some(old) = data.imp().binding.replace(Some(binding)) {
                        old.unbind();
                    }
                }

                let this = upgrade!(this);
                this.selected_item_changed(list_item);
            });

            factory.connect_unbind(move |_factory, item| {
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let data = list_item.item().and_downcast::<super::DropData>().unwrap();
                if let Some(old) = data.imp().binding.replace(None) {
                    old.unbind();
                }
            });
            factory
        }

        fn selected_item_changed(&self, list_item: &gtk::ListItem) {
            let Some(box_item) = list_item.child().and_downcast::<gtk::Box>() else {
                return;
            };

            let image = box_item.first_child().and_downcast::<gtk::Image>().unwrap();

            let selected = list_item
                .item()
                .and_downcast_ref::<super::DropData>()
                .map(|s| s.selected())
                .unwrap_or_default();
            let opacity = if selected { 1.0 } else { 0.0 };
            image.set_opacity(opacity);
        }

        fn set_popup_visible(&self, visible: bool) {
            if visible {
                self.popover.popup();
                self.search_entry.grab_focus();
            } else {
                self.popover.popdown();
            }
        }

        #[template_callback]
        fn row_activated_cb(&self, position: u32) {
            self.set_popup_visible(false);
            self.clear_filter();

            let popup_position = self.popup_selection_model().selected();

            info!("Item pos filt {} pos pop {}", position, popup_position);

            self.set_selected(popup_position);
        }

        fn set_selected(&self, position: u32) {
            let binding = self.popup_selection_model().item(position);

            //if equals you remove
            let data =
                if binding.as_ref() == self.selected.borrow().and_upcast_ref::<glib::Object>() {
                    self.obj().set_subtitle("");
                    None
                } else {
                    let data = binding.and_downcast_ref::<super::DropData>();

                    debug!("pos {position} {:?} {:?}", binding, data);

                    if let Some(data) = data {
                        data.set_selected(true);
                        self.obj().set_subtitle(&data.string());
                    }
                    data
                };

            if let Some(old) = self.selected.replace(data.cloned()) {
                old.set_selected(false);
            }
        }

        fn clear_filter(&self) {
            self.search_entry.set_text("");
        }

        fn set_enable_search(&self, enable: bool) {
            self.search_entry.set_visible(enable)
        }

        fn enable_search(&self) -> bool {
            self.search_entry.is_visible()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MyDropdownImp {
        const NAME: &'static str = "MyDropDown";
        type Type = super::MyDropDown;
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
    impl ObjectImpl for MyDropdownImp {
        fn constructed(&self) {
            self.parent_constructed();

            let factory = self.create_default_factory();

            self.list.set_factory(Some(&factory));
            self.factory.replace(Some(factory.into()));
            // self.drop_list_view()
            // .set_model(Some(self.popup_selection_model()));

            let this = self.obj().downgrade();
            let gesture = gtk::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                let this = upgrade!(this);
                let visible = this.imp().popover.is_visible();
                this.imp().set_popup_visible(!visible);
            });

            self.obj().add_controller(gesture);

            //to highlight on hover
            self.obj().set_activatable(true);

            // self.obj().connect_activate(|s| {
            //     s.imp().popover.popup();
            // });
            // self.current.set_model(Some(self.current_model()));
            self.old_selected.set(GTK_INVALID_LIST_POSITION)
        }
    }

    impl WidgetImpl for MyDropdownImp {}
    impl ListBoxRowImpl for MyDropdownImp {}
    impl PreferencesRowImpl for MyDropdownImp {}
    impl ActionRowImpl for MyDropdownImp {}
}
