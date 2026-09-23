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
    use gtk::{ffi::GTK_INVALID_LIST_POSITION, prelude::*};
    use tracing::{debug, error, info};

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

        // #[template_child]
        // current: TemplateChild<gtk::ListView>,
        filter_list_model: OnceCell<gtk::FilterListModel>,

        selection_model: OnceCell<gtk::SingleSelection>,

        popup_selection_model: OnceCell<gtk::SingleSelection>,

        // current_model: OnceCell<gtk::NoSelection>,
        last_filter_string: RefCell<String>,

        custom_filter: OnceCell<gtk::CustomFilter>,

        search_entry: OnceCell<gtk::SearchEntry>,

        popover: OnceCell<gtk::Popover>,

        drop_list_view: OnceCell<gtk::ListView>,

        factory: RefCell<Option<gtk::ListItemFactory>>,
    }

    impl SysDDropdownImp {
        fn popup_selection_model(&self) -> &gtk::SingleSelection {
            if let Some(model) = self.popup_selection_model.get() {
                model
            } else {
                let filter_model = self.filter_list_model();
                let single_selection = gtk::SingleSelection::builder()
                    .model(filter_model)
                    .autoselect(false)
                    .can_unselect(true)
                    .build();
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

        fn selection_model(&self) -> &gtk::SingleSelection {
            if let Some(model) = self.selection_model.get() {
                model
            } else {
                let single_selection = gtk::SingleSelection::builder()
                    .autoselect(false)
                    .can_unselect(true)
                    .build();
                let this = self.downgrade();
                single_selection.connect_selected_notify(move |selection| {
                    let this = upgrade!(this);
                    this.selection_changed(selection);
                });
                let this = self.downgrade();
                single_selection.connect_selected_item_notify(move |selection| {
                    let this = upgrade!(this);
                    this.selection_item_changed(selection);
                });
                let this = self.downgrade();
                single_selection.connect_items_changed(move |selection, p, r, a| {
                    let this = upgrade!(this);
                    this.model_changed(selection, p, r, a);
                });

                let _ = self.selection_model.set(single_selection);
                self.selection_model.get().unwrap()
            }
        }

        fn selection_changed(&self, selection: &gtk::SingleSelection) {
            info!("selection change");

            let selected = selection.selected();

            self.clear_filter();

            self.popup_selection_model().set_selected(selected);
        }

        fn selection_item_changed(&self, selection: &gtk::SingleSelection) {
            info!("selection item change");

            let value = selection
                .selected_item()
                .and_downcast_ref::<gtk::StringObject>()
                .map(|so| so.string());

            self.obj().set_subtitle(&value.unwrap_or_default());
        }

        fn model_changed(
            &self,
            selection: &gtk::SingleSelection,
            position: u32,
            removed: u32,
            added: u32,
        ) {
            info!(
                "model change. pos {position} rem {removed} add {added}, total {}",
                selection.n_items()
            );

            selection.set_selected(GTK_INVALID_LIST_POSITION);
        }

        pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
            self.filter_list_model().set_model(model);
            self.selection_model().set_model(model);
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

        fn create_filter(&self) -> &gtk::CustomFilter {
            if let Some(filter) = self.custom_filter.get() {
                filter
            } else {
                let search_entry = self.search_entry().clone();

                let filter = gtk::CustomFilter::new(move |object| {
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
                });
                let _ = self.custom_filter.set(filter);
                self.custom_filter.get().unwrap()
            }
        }

        fn popover(&self) -> &gtk::Popover {
            let this = self.downgrade();
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
                pop.set_parent(&this.arrow_down_image.get());

                pop.connect_visible_notify(move |p| {
                    info!("pop up visible {}", p.is_visible());

                    //FIXME UGLY WORK AROUND
                    if p.get_visible() {
                        if let Some(f) = this.factory.borrow().as_ref() {
                            this.drop_list_view().set_factory(Some(f));
                        }
                    } else {
                        if let Some(f) = this.drop_list_view().factory() {
                            this.factory.replace(Some(f));
                        };

                        this.drop_list_view()
                            .set_factory(None::<&gtk::ListItemFactory>);
                    }
                });
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
                    // .tab_behavior(gtk::ListTabBehavior::Item)
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

                let this = self.downgrade();
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

                    let value = this.clone();

                    let handler = list_item.connect_selected_notify(move |list_item| {
                        let value = upgrade!(value);
                        value.selected_item_changed(list_item);
                    });
                    // let handler = list_item.connect_selectable_notify(move |list_item| {
                    //     value.imp().selected_item_changed(list_item);
                    // });
                    unsafe { list_item.set_qdata(list_item_quark(), handler) };

                    let this = upgrade!(this);
                    this.selected_item_changed(list_item);

                    let handler = item_box.connect_root_notify(move |d| {
                        this.root_changed(d);
                    });
                    unsafe { item_box.set_qdata(list_item_quark(), handler) };
                });

                factory.connect_unbind(|_factory, item| {
                    let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();

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
            info!("selectied item change");
            let box_item = list_item.child().and_downcast::<gtk::Box>().unwrap();
            let image = box_item.first_child().and_downcast::<gtk::Image>().unwrap();

            let opacity = if self.get_selected_item() == list_item.item() {
                1.0
            } else {
                0.0
            };
            image.set_opacity(opacity);
        }

        fn root_changed(&self, bbox: &gtk::Box) {
            // info!("root changed");

            let Some(icon) = bbox.first_child() else {
                return;
            };

            if bbox.ancestor(gtk::Popover::static_type()).as_ref()
                == Some(self.popover().upcast_ref::<gtk::Widget>())
            {
                icon.set_visible(true);
            } else {
                icon.set_visible(false);
            }
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

        fn row_activated(&self, _list: &gtk::ListView, position: u32) {
            self.popover().popdown();

            self.clear_filter();

            let popup_position = self.popup_selection_model().selected();

            info!("Item pos filt {} pos pop {}", position, popup_position);

            self.set_selected(popup_position);
        }

        fn set_selected(&self, position: u32) {
            let position = if self.selection_model().selected() == position {
                GTK_INVALID_LIST_POSITION
            } else {
                position
            };

            self.selection_model().set_selected(position);
        }

        fn clear_filter(&self) {
            self.search_entry().set_text("");
        }

        fn get_selected_item(&self) -> Option<glib::Object> {
            self.selection_model().selected_item()
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

            self.drop_list_view()
                .set_model(Some(self.popup_selection_model()));

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

            let this = self.downgrade();
            self.drop_list_view().connect_activate(move |list, active| {
                info!("position {}", active);
                let this = upgrade!(this);

                this.row_activated(list, active)
            });

            self.obj().connect_activate(|s| {
                s.imp().popover().popup();
            });
            // self.current.set_model(Some(self.current_model()));
        }
    }

    impl WidgetImpl for SysDDropdownImp {}
    impl ListBoxRowImpl for SysDDropdownImp {}
    impl PreferencesRowImpl for SysDDropdownImp {}
    impl ActionRowImpl for SysDDropdownImp {}
}
