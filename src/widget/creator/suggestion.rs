//test

use glib::{object::IsA, subclass::types::ObjectSubclassIsExt};
use tracing::warn;

glib::wrapper! {
    pub struct SuggestionRow(ObjectSubclass<imp::SuggestionRowImp>)
    @extends adw::EntryRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable,  gtk::Buildable,  gtk::ConstraintTarget, gtk::Editable ;
}

impl SuggestionRow {
    pub fn new() -> Self {
        let obj: SuggestionRow = glib::Object::new();
        obj
    }

    pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
        self.imp().set_model(model)
    }

    pub fn set_factory(&self, factory: Option<&impl IsA<gtk::ListItemFactory>>) {
        self.imp().set_factory(factory);
    }

    pub fn set_text2(&self, text: &str) {
        self.imp().set_text2(text)
    }

    pub fn set_expression(&self, expression: gtk::PropertyExpression) {
        let _ = self
            .imp()
            .expression
            .set(expression)
            .inspect_err(|_| warn!("Expression already set!"));
    }
}

impl Default for SuggestionRow {
    fn default() -> Self {
        SuggestionRow::new()
    }
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::{
        object::IsA,
        subclass::{object::ObjectImpl, types::ObjectSubclass},
    };
    use gtk::{
        gdk::{self, Key},
        prelude::*,
    };
    use std::cell::{Cell, OnceCell, RefCell};
    use tracing::{debug, error, info};

    use crate::{upgrade, widget::find_child_by_name};

    const PAGE_STEP: u32 = 10;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::SuggestionRow)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/suggestion_entry.ui")]
    // #[properties(wrapper_type = super::SDDropdown)]
    pub struct SuggestionRowImp {
        drop_list_view: OnceCell<gtk::ListView>,

        popover: OnceCell<gtk::Popover>,

        #[template_child]
        arrow_down_image: TemplateChild<gtk::Image>,

        #[property(get, set)]
        popup_visible: Cell<bool>,

        filter_list_model: OnceCell<gtk::FilterListModel>,

        single_selection: OnceCell<gtk::SingleSelection>,

        search: RefCell<String>,

        change_id: OnceCell<glib::SignalHandlerId>,

        custom_filter: OnceCell<gtk::CustomFilter>,

        pub(super) expression: OnceCell<gtk::PropertyExpression>,
    }

    impl SuggestionRowImp {
        pub fn set_model(&self, model: Option<&impl IsA<gio::ListModel>>) {
            if let Some(fl) = self.filter_list_model.get() {
                fl.set_model(model);
            }
        }

        fn create_filter(&self) -> gtk::CustomFilter {
            let this = self.obj().downgrade();

            // let expression = self.expression.get().clone();

            gtk::CustomFilter::new(move |object| {
                let this = upgrade!(this, false);
                let text_gs = this.text();
                if text_gs.is_empty() {
                    return true;
                }

                let Some(expression) = this.imp().expression.get() else {
                    return true;
                };

                let Some(value) = expression.evaluate(Some(object)).and_then(|value| {
                    value
                        .get::<String>()
                        .inspect_err(|err| error!("bad convertion {:?}", err))
                        .ok()
                }) else {
                    return true;
                };

                let text = text_gs.as_str();

                //if an upper case --> filter
                if text_gs.chars().any(|c| c.is_ascii_uppercase()) {
                    value.contains(text)
                } else {
                    value.to_ascii_lowercase().contains(text)
                }
            })
        }

        pub fn set_factory(&self, factory: Option<&impl IsA<gtk::ListItemFactory>>) {
            self.drop_list_view().set_factory(factory);
        }

        fn set_popup_visible(&self, visible: bool) {
            if visible {
                if let Some(single_selection) = self.single_selection.get() {
                    single_selection.set_selected(gtk::INVALID_LIST_POSITION);
                }
                self.popover().popup();
            } else {
                self.popover().popdown();
            }

            // Set the porperty indicator
            self.obj().set_popup_visible(visible);
        }

        fn drop_list_view(&self) -> &gtk::ListView {
            self.drop_list_view
                .get_or_init(|| gtk::ListView::builder().build())
        }

        fn popover(&self) -> &gtk::Popover {
            let this_weak = self.obj().downgrade();
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

                let this = upgrade!(this_weak, pop);
                pop.set_parent(&this.imp().arrow_down_image.get());

                let scroll = gtk::ScrolledWindow::new();
                scroll.set_child(Some(this.imp().drop_list_view()));
                pop.set_child(Some(&scroll));
                pop
            })
        }

        fn key_pressed(
            &self,
            _controller: &gtk::EventControllerKey,
            key: gdk::Key,
            _keycode: u32,
            state: gdk::ModifierType,
        ) -> glib::Propagation {
            if state.contains(
                gdk::ModifierType::SHIFT_MASK
                    | gdk::ModifierType::ALT_MASK
                    | gdk::ModifierType::CONTROL_MASK,
            ) {
                self.accept_current_selection();
                return glib::Propagation::Stop;
            }

            match key {
                Key::Return | Key::KP_Enter | Key::ISO_Enter => {
                    self.accept_current_selection();
                    let visible = self.popup_visible.get();
                    self.text_changed_idle(false);

                    info!("Enter: {visible}");
                    self.set_popup_visible(!visible);
                    glib::Propagation::Proceed
                }
                Key::Escape => {
                    self.set_popup_visible(false);
                    let handler_id = self.change_id.get().unwrap();

                    let this = self.obj();
                    this.block_signal(handler_id);
                    this.set_text("");
                    this.set_position(-1);
                    self.set_popup_visible(false);
                    self.text_changed_idle(false);
                    this.unblock_signal(handler_id);
                    glib::Propagation::Stop
                }
                Key::Tab | Key::KP_Tab | Key::ISO_Left_Tab => {
                    self.set_popup_visible(false);
                    glib::Propagation::Proceed
                }
                _ => {
                    let Some(single) = self.single_selection.get() else {
                        return glib::Propagation::Stop;
                    };

                    let matches = single.n_items();
                    let mut selected = single.selected();

                    let proceed = match key {
                        Key::Up | Key::KP_Up => {
                            if selected == 0 {
                                selected = gtk::INVALID_LIST_POSITION;
                            } else if selected == gtk::INVALID_LIST_POSITION {
                                selected = matches - 1;
                            } else {
                                selected -= 1;
                            }

                            glib::Propagation::Stop
                        }
                        Key::Down | Key::KP_Down => {
                            if selected == matches - 1 {
                                selected = gtk::INVALID_LIST_POSITION;
                            } else if selected == gtk::INVALID_LIST_POSITION {
                                selected = 0;
                            } else {
                                selected += 1;
                            }

                            glib::Propagation::Stop
                        }
                        Key::Page_Up => {
                            if selected == 0 {
                                selected = gtk::INVALID_LIST_POSITION;
                            } else if selected == gtk::INVALID_LIST_POSITION {
                                selected = matches - 1;
                            } else if selected >= PAGE_STEP {
                                selected -= PAGE_STEP
                            } else {
                                selected -= 1;
                            }

                            glib::Propagation::Stop
                        }
                        Key::Page_Down => {
                            if selected == matches - 1 {
                                selected = gtk::INVALID_LIST_POSITION;
                            } else if selected == gtk::INVALID_LIST_POSITION {
                                selected = 0;
                            } else if selected + PAGE_STEP < matches {
                                selected += PAGE_STEP
                            } else {
                                selected += 1;
                            }

                            glib::Propagation::Stop
                        }
                        _ => glib::Propagation::Proceed,
                    };

                    if selected != gtk::INVALID_LIST_POSITION {
                        self.drop_list_view().scroll_to(
                            selected,
                            gtk::ListScrollFlags::SELECT,
                            None,
                        );
                    }
                    proceed
                }
            }
        }

        fn accept_current_selection(&self) {
            let Some(item) = self.single_selection.get().and_then(|s| s.selected_item()) else {
                return;
            };

            let handler_id = self.change_id.get().unwrap();

            self.obj().block_signal(handler_id);

            let Some(expression) = self.expression.get() else {
                error!("Suggestion Expression None");
                return;
            };

            if let Some(value) = expression
                .evaluate(Some(&item))
                .and_then(|v| v.get::<String>().ok())
            {
                self.obj().set_text(&value);
            }

            self.obj().set_position(-1);

            self.obj().unblock_signal(handler_id);
        }

        fn text_changed_idle(&self, manage_popup: bool) {
            let text = self.obj().text();

            let mut last_filter = self.search.borrow_mut();

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

            if manage_popup {
                let matches = self
                    .single_selection
                    .get()
                    .map(|s| s.n_items())
                    .unwrap_or_default();

                self.set_popup_visible(matches > 0);
            }
        }

        fn text_changed(&self) {
            /* We need to defer to an idle since GtkText sets selection bounds
             * after notify::text
             */
            let this = self.obj().downgrade();
            glib::spawn_future_local(async move {
                let this = upgrade!(this);
                this.imp().text_changed_idle(true);
            });
        }

        pub(crate) fn set_text2(&self, text: &str) {
            let handler_id = self.change_id.get().unwrap();

            let this = self.obj();
            this.block_signal(handler_id);
            this.set_text(text);
            this.unblock_signal(handler_id);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SuggestionRowImp {
        const NAME: &'static str = "SuggestionRow";
        type Type = super::SuggestionRow;
        type ParentType = adw::EntryRow;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SuggestionRowImp {
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

            let _ = self.single_selection.set(selection_model.clone());

            let _ = self.filter_list_model.set(filter_list_model);
            let this = self.obj().downgrade();

            let gesture = gtk::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                let this = upgrade!(this);
                let visible = this.popup_visible();
                this.imp().set_popup_visible(!visible);
            });

            self.obj().add_controller(gesture);

            let controller = gtk::EventControllerKey::new();

            let this = self.obj().downgrade();
            controller.connect_key_pressed(move |controller_key, key, code, status| {
                let this = upgrade!(this, glib::Propagation::Proceed);
                this.imp().key_pressed(controller_key, key, code, status)
            });

            let this = self.obj().clone();
            if let Some(text) = find_child_by_name::<gtk::Text>(&this, "text") {
                text.add_controller(controller);
            } else {
                error!("Text Widget not found");
            }

            selection_model.connect_selected_notify(move |_| {
                this.imp().accept_current_selection();
            });

            let handler = self
                .obj()
                .connect_text_notify(|this| this.imp().text_changed());

            let _ = self.change_id.set(handler);
            self.drop_list_view().set_model(Some(&selection_model));

            let controller = gtk::EventControllerFocus::new();

            let this = self.obj().clone();
            controller.connect_leave(move |_controller| {
                this.imp().set_popup_visible(false);
                this.imp().accept_current_selection();
            });
            self.obj().add_controller(controller);
        }
    }

    impl WidgetImpl for SuggestionRowImp {}
    impl ListBoxRowImpl for SuggestionRowImp {}
    impl PreferencesRowImpl for SuggestionRowImp {}
    impl EntryRowImpl for SuggestionRowImp {}
}
