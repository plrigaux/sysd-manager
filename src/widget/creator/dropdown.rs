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
    use std::{
        cell::{OnceCell, RefCell},
        sync::OnceLock,
    };

    use adw::{prelude::ActionRowExt, subclass::prelude::*};
    use gettextrs::gettext;
    use glib::{
        Quark,
        object::{Cast, CastNone, IsA},
        subclass::{object::ObjectImpl, types::ObjectSubclass},
    };
    use gtk::prelude::*;
    use tracing::{debug, error, info, warn};

    use crate::upgrade;

    static BOX: OnceLock<Quark> = OnceLock::new();

    fn box_quark() -> Quark {
        *BOX.get_or_init(|| Quark::from_str("Box_h"))
    }
    static LIST_ITEM: OnceLock<Quark> = OnceLock::new();

    fn list_item_quark() -> Quark {
        *LIST_ITEM.get_or_init(|| Quark::from_str("li_h"))
    }

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/dropdown.ui")]
    pub struct SysDDropdownImp {
        #[template_child]
        arrow_down_image: TemplateChild<gtk::Image>,

        filter_list_model: OnceCell<gtk::FilterListModel>,

        selection_model: OnceCell<gtk::SingleSelection>,

        last_filter_string: RefCell<String>,

        custom_filter: OnceCell<gtk::CustomFilter>,

        search_entry: OnceCell<gtk::SearchEntry>,

        popover: OnceCell<gtk::Popover>,

        drop_list_view: OnceCell<gtk::ListView>,
    }

    impl SysDDropdownImp {
        pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
            if let Some(fl) = self.filter_list_model.get() {
                fl.set_model(model);
            }
        }

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
            let this = self.obj().downgrade();
            self.popover.get_or_init(|| {
                let pop = gtk::Popover::builder()
                    .css_classes(["menu"])
                    // .autohide(false) //for not loosing the focus
                    .autohide(true)
                    .has_arrow(true)
                    .height_request(300)
                    .width_request(200)
                    .position(gtk::PositionType::Bottom)
                    // .can_focus(false)
                    .build();

                let this = upgrade!(this, pop);
                pop.set_parent(&this.imp().arrow_down_image.get());

                pop.connect_visible_notify(|p| info!("pop up visible {}", p.is_visible()));
                let boxx = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .build();
                boxx.append(self.search_entry());

                let scroll = gtk::ScrolledWindow::builder()
                    .child(self.drop_list_view())
                    .max_content_height(400)
                    .propagate_natural_height(true)
                    .propagate_natural_width(true)
                    .build();

                boxx.append(&scroll);
                pop.set_child(Some(&boxx));
                pop
            })
        }

        fn drop_list_view(&self) -> &gtk::ListView {
            self.drop_list_view.get_or_init(|| {
                let drop_list_view = gtk::ListView::builder()
                    .single_click_activate(true)
                    .tab_behavior(gtk::ListTabBehavior::Item)
                    .build();

                let factory = gtk::SignalListItemFactory::new();
                factory.connect_setup(move |_factory, item| {
                    let item = item.downcast_ref::<gtk::ListItem>().unwrap();
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

                    item.set_child(Some(&item_box));
                });

                let this = self.obj().downgrade();
                factory.connect_bind(move |_factory, item| {
                    let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                    let data = list_item
                        .item()
                        .and_downcast::<gtk::StringObject>()
                        .unwrap();

                    let item_box = list_item.child().and_downcast::<gtk::Box>().unwrap();
                    if let Some(label) = item_box.last_child().and_downcast_ref::<gtk::Label>() {
                        label.set_label(&data.string());
                    };

                    // let dd =     item_box.connect(signal_name, after, callback);
                    //
                    let this = upgrade!(this);
                    let value = this.clone();

                    let handler = list_item.connect_selected_notify(move |list| {
                        value.imp().selected_item_changed(list);
                    });

                    // let list_item2 = list_item.clone();
                    // let handler = list_item.connect("notify::selected-item", false, move |list| {
                    //     Self::selected_item_changed(&value, &list_item2);
                    //     None
                    // });
                    //
                    // let handler = list_item.connect_closure(
                    //     "notify::selected-item",
                    //     false,
                    //     glib::closure_local!(move || {
                    //         Self::selected_item_changed(&value, &list_item2);
                    //     }),
                    // );

                    unsafe { list_item.set_qdata(list_item_quark(), handler) };
                    // let handler = item_box.connect_root_notify(|boxo| {});
                    // unsafe { item_box.set_qdata(box_quark(), handler) };
                    this.imp().selected_item_changed(list_item);
                });

                factory.connect_unbind(|_factory, item| {
                    let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                    // let data = item.item().and_downcast::<gtk::StringObject>().unwrap();

                    let box_item = list_item.child().and_downcast::<gtk::Box>().unwrap();

                    if let Some(handler_id) = unsafe { list_item.steal_qdata(list_item_quark()) } {
                        list_item.disconnect(handler_id);
                    }

                    if let Some(handler_id) = unsafe { box_item.steal_qdata(box_quark()) } {
                        box_item.disconnect(handler_id);
                    }
                });

                drop_list_view.set_factory(Some(&factory));
                drop_list_view
            })
        }

        fn selected_item_changed(&self, list_item: &gtk::ListItem) {
            let box_item = list_item.child().and_downcast::<gtk::Box>().unwrap();
            let image = box_item.first_child().and_downcast::<gtk::Image>().unwrap();

            // let string_object = list_item
            //     .item()
            //     .and_downcast::<gtk::StringObject>()
            //     .unwrap();

            let opacity = if self.get_selected_item() == list_item.item() {
                // let opacity = if this.subtitle() == Some(string_object.string()) {
                1.0
            } else {
                0.0
            };
            image.set_opacity(opacity);
        }

        fn search_entry(&self) -> &gtk::SearchEntry {
            self.search_entry.get_or_init(|| {
                let search = gtk::SearchEntry::builder()
                    // Translators: placeholder text of the search entry from Custom AdwComboRow.
                    // It should be phrased as a verb
                    .placeholder_text(gettext("Search"))
                    .css_classes(["combo-searchbar"])
                    .width_chars(6)
                    .max_width_chars(6)
                    .margin_end(10)
                    .margin_start(10)
                    .margin_top(10)
                    .build();

                let this = self.obj().downgrade();
                search.connect_search_changed(move |search_entry| {
                    let this = upgrade!(this);
                    this.imp().search_entry_changed(search_entry);
                });
                search.connect_stop_search(|_s| info!("Search stop - do nothing"));
                search
            })
        }

        fn set_popup_visible(&self, visible: bool) {
            if visible {
                self.popover().popup();
                self.search_entry().grab_focus();
            } else {
                self.popover().popdown();
            }
        }

        fn list_activate(&self, _list: &gtk::ListView, position: u32) {
            self.popover().popdown();

            let Some(model) = self.filter_list_model.get() else {
                warn!("No filter model");
                return;
            };

            let Some(some) = model.item(position).and_downcast::<gtk::StringObject>() else {
                warn!("No item at {}", position);
                return;
            };

            self.obj().set_subtitle(&some.string());
            self.search_entry().set_text("");

            let x = self.selection_model.get().unwrap().selected();

            info!("Item {} pos f {} x {}", some.string(), position, x);
        }

        fn get_selected_item(&self) -> Option<glib::Object> {
            let single_selection = self.selection_model.get().unwrap();
            single_selection.selected_item()
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
            self.drop_list_view().set_model(Some(&selection_model));
            let _ = self.selection_model.set(selection_model);

            // selection_model.connect_selected_notify(move |selection| {
            //     if let Some(item) = selection
            //         .selected_item()
            //         .and_downcast_ref::<gtk::StringObject>()
            //     {
            //         info!("select item {}", item.string());
            //     }
            // });

            let this = self.obj().downgrade();
            let gesture = gtk::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                let this = upgrade!(this);
                let visible = this.imp().popover().is_visible();
                this.imp().set_popup_visible(!visible);
            });

            self.obj().add_controller(gesture);

            //to highlight on hover
            self.obj().set_activatable(true);

            let this = self.obj().downgrade();
            self.drop_list_view().connect_activate(move |list, active| {
                info!("position {}", active);
                let this = upgrade!(this);

                this.imp().list_activate(list, active)
            });
        }
    }

    impl WidgetImpl for SysDDropdownImp {}
    impl ListBoxRowImpl for SysDDropdownImp {}
    impl PreferencesRowImpl for SysDDropdownImp {}
    impl ActionRowImpl for SysDDropdownImp {}
}
