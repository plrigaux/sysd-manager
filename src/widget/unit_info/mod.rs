use crate::systemd::data::UnitInfo;

mod construct_info;
mod time_handling;
mod writer;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

glib::wrapper! {
    pub struct UnitInfoPanel(ObjectSubclass<imp::UnitInfoPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitInfoPanel {
    pub fn new(is_dark: bool) -> Self {
        // Create new window
        let obj: UnitInfoPanel = glib::Object::new();

        obj.set_dark(is_dark);

        obj
    }

    pub fn display_unit_info(&self, unit: &UnitInfo) {
        self.imp().display_unit_info(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark)
    }

    fn hovering_over_link(&self) -> bool {
        self.imp().hovering_over_link.get()
    }

    fn set_hovering_over_link(&self, hovering_over_link: bool) {
        self.imp().hovering_over_link.set(hovering_over_link);
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::{
        gdk, gio,
        glib::{self, Value},
        prelude::*,
        subclass::{
            box_::BoxImpl,
            prelude::*,
            widget::{
                CompositeTemplateCallbacksClass, CompositeTemplateClass,
                CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
            },
        },
        FileLauncher, TemplateChild,
    };

    use log::{info, warn};

    use crate::{systemd::data::UnitInfo, widget::info_window::InfoWindow};

    use super::{
        construct_info::fill_all_info,
        writer::{UnitInfoWriter, TAG_DATA_LINK},
        UnitInfoPanel,
    };

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_info_panel.ui")]
    pub struct UnitInfoPanelImp {
        #[template_child]
        show_all_button: TemplateChild<gtk::Button>,

        #[template_child]
        refresh_button: TemplateChild<gtk::Button>,

        #[template_child]
        unit_info_textview: TemplateChild<gtk::TextView>,

        unit: RefCell<Option<UnitInfo>>,

        is_dark: Cell<bool>,

        pub hovering_over_link: Cell<bool>,
    }

    #[gtk::template_callbacks]
    impl UnitInfoPanelImp {
        #[template_callback]
        fn refresh_info_clicked(&self, button: &gtk::Button) {
            info!("button {:?}", button);

            let binding = self.unit.borrow();
            let Some(unit) = binding.as_ref() else {
                warn!("no unit file");
                return;
            };

            self.update_unit_info(&unit)
        }

        #[template_callback]
        fn show_all_clicked(&self, _button: &gtk::Button) {
            let binding = self.unit.borrow();
            let Some(unit) = binding.as_ref() else {
                warn!("no unit file");
                return;
            };

            let info_window = InfoWindow::new();

            info!("show_all_clicked {:?}", unit.primary());

            info_window.fill_data(&unit);

            info_window.present();
        }

        pub(crate) fn display_unit_info(&self, unit: &UnitInfo) {
            let _old = self.unit.replace(Some(unit.clone()));

            self.update_unit_info(&unit)
        }

        /// Updates the associated journal `TextView` with the contents of the unit's journal log.
        fn update_unit_info(&self, unit: &UnitInfo) {
            let unit_info_text_view: &gtk::TextView = self.unit_info_textview.as_ref();

            let buf = unit_info_text_view.buffer();

            buf.set_text(""); // clear text

            let start_iter = buf.start_iter();

            let is_dark = self.is_dark.get();

            let mut info_writer = UnitInfoWriter::new(buf, start_iter, is_dark);

            fill_all_info(unit, &mut info_writer);

            //buf.insert_markup(&mut start_iter, &text);
        }

        pub(crate) fn set_dark(&self, is_dark: bool) {
            self.is_dark.set(is_dark);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for UnitInfoPanelImp {
        const NAME: &'static str = "UnitInfoPanel";
        type Type = super::UnitInfoPanel;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnitInfoPanelImp {
        fn constructed(&self) {
            self.parent_constructed();
            {
                let text_view = self.unit_info_textview.clone();
                let event_controller_key = gtk::EventControllerKey::new();

                event_controller_key.connect_key_pressed(
                    move |_event_controller_key, keyval: gdk::Key, _keycode, _modifiers| {
                        match keyval {
                            gdk::Key::Return | gdk::Key::KP_Enter => {
                                let buffer = text_view.buffer();
                                let mark = buffer.get_insert();
                                let iter = buffer.iter_at_mark(&mark);

                                follow_if_link(iter);
                            }
                            _ => {}
                        }
                        glib::Propagation::Proceed
                    },
                );
                self.unit_info_textview.add_controller(event_controller_key);
            }

            {
                let event_controller_motion = gtk::EventControllerMotion::new();

                let text_view = self.unit_info_textview.clone();
                let info_panel = self.obj().clone();
                event_controller_motion.connect_motion(move |_motion_ctl, x, y| {
                    let (tx, ty) = text_view.window_to_buffer_coords(
                        gtk::TextWindowType::Widget,
                        x as i32,
                        y as i32,
                    );

                    set_cursor_if_appropriate(&info_panel, &text_view, tx, ty);
                });
                self.unit_info_textview
                    .add_controller(event_controller_motion);
            }

            {
                let gesture_click = gtk::GestureClick::new();
                let text_view = self.unit_info_textview.clone();
                gesture_click.connect_released(move |_gesture_click, _n_press, x, y| {
                    let buf = text_view.buffer();

                    // we shouldn't follow a link if the user has selected something
                    if let Some((start, end)) = buf.selection_bounds() {
                        if start.offset() != end.offset() {
                            return;
                        }
                    }

                    let Some(iter) = text_view.iter_at_location(x as i32, y as i32) else {
                        return;
                    };

                    follow_if_link(iter);
                });

                self.unit_info_textview.add_controller(gesture_click);
            }
        }
    }
    impl WidgetImpl for UnitInfoPanelImp {}
    impl BoxImpl for UnitInfoPanelImp {}

    fn set_cursor_if_appropriate(
        info_panel: &UnitInfoPanel,
        text_view: &gtk::TextView,
        x: i32,
        y: i32,
    ) {
        let mut hovering = false;
        if let Some(iter) = text_view.iter_at_location(x, y) {
            let tags = iter.tags();

            for tag in tags.iter() {
                let val = unsafe {
                    let val: Option<std::ptr::NonNull<Value>> = tag.data(TAG_DATA_LINK);
                    val
                };

                if val.is_some() {
                    hovering = true;
                    break;
                }
            }
        }

        if info_panel.hovering_over_link() != hovering {
            info_panel.set_hovering_over_link(hovering);
            if hovering {
                text_view.set_cursor_from_name(Some("pointer"));
            } else {
                text_view.set_cursor_from_name(Some("text"));
            }
        }
    }

    fn follow_if_link(iter: gtk::TextIter) {
        let tags = iter.tags();

        let mut link_value_op = None;

        for tag in tags.iter() {
            //info!("TAG {:?} {:?}", tag, tag.name());

            link_value_op = unsafe {
                let val: Option<std::ptr::NonNull<Value>> = tag.data(TAG_DATA_LINK);
                if let Some(link_value_nonull) = val {
                    Some(link_value_nonull.as_ref())
                } else {
                    None
                }
            };

            if link_value_op.is_some() {
                break;
            }
        }

        if let Some(link_value) = link_value_op {
            match link_value.get::<String>() {
                Ok(file_link) => {
                    let uri = format!("file://{}", file_link);

                    let file = gio::File::for_uri(&uri);
                    let launcher = FileLauncher::new(Some(&file));
                    launcher.launch(
                        None::<&gtk::Window>,
                        None::<&gio::Cancellable>,
                        move |result| {
                            if let Err(error) = result {
                                warn!("Finished launch {} Error {:?}", uri, error)
                            }
                        },
                    );
                }
                Err(e) => warn!("Link value Error {:?}", e),
            }
        }
    }
}
