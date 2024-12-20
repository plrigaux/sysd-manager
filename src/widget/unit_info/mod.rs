use crate::systemd::data::UnitInfo;

mod construct_info;
mod time_handling;

use gtk::{
    glib, pango,
    prelude::{TextBufferExt, TextBufferExtManual, ToValue},
    subclass::prelude::ObjectSubclassIsExt,
    TextBuffer, TextIter, TextTag,
};

use super::journal::palette::Palette;
use crate::gtk::glib::translate::IntoGlib;

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
}

struct UnitInfoWriter {
    buf: TextBuffer,
    iter: TextIter,
    is_dark: bool,
}

const TAG_NAME_HYPER_LINK: &str = "hyperlink";
const TAG_NAME_ACTIVE: &str = "active";
const TAG_NAME_DISABLE: &str = "disable";

impl UnitInfoWriter {
    fn insert(&mut self, text: &str) {
        self.buf.insert(&mut self.iter, text);
    }

    fn new_line(&mut self) {
        self.buf.insert(&mut self.iter, "\n");
    }

    fn insert_active(&mut self, text: &str) {
        self.insert_tag(text, Self::create_active_tag);
    }

    fn insert_disable(&mut self, text: &str) {
        self.insert_tag(text, Self::create_disable_tag);
    }

    fn hyper_link(&mut self, text: &str, _link: &str) {
        //   self.buf.insert(&mut self.iter, text);
        self.insert_tag(text, Self::create_hyperlink_tag);
    }

    fn create_hyperlink_tag(buf: &TextBuffer, _is_dark: bool) -> Option<TextTag> {
        let tag_op = buf.tag_table().lookup(TAG_NAME_HYPER_LINK);
        if tag_op.is_some() {
            return tag_op;
        }

        let tag_op = buf.create_tag(
            Some(TAG_NAME_HYPER_LINK),
            &[
                ("foreground", &"blue".to_value()),
                ("underline", &pango::Underline::Single.to_value()),
            ],
        );

        tag_op
    }

    fn create_active_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let tag_op = buf.tag_table().lookup(TAG_NAME_ACTIVE);
        if tag_op.is_some() {
            return tag_op;
        }

        let color = if is_dark {
            Palette::Green3.get_color()
        } else {
            Palette::Green5.get_color()
        };

        let tag_op = buf.create_tag(
            Some(TAG_NAME_ACTIVE),
            &[
                ("foreground", &color.to_value()),
                ("weight", &pango::Weight::Bold.into_glib().to_value()),
            ],
        );

        tag_op
    }

    fn create_disable_tag(buf: &TextBuffer, is_dark: bool) -> Option<TextTag> {
        let tag_op = buf.tag_table().lookup(TAG_NAME_DISABLE);
        if tag_op.is_some() {
            return tag_op;
        }

        let color = if is_dark {
            Palette::Yellow3.get_color()
        } else {
            Palette::Yellow3.get_color()
        };

        let tag_op = buf.create_tag(
            Some(TAG_NAME_DISABLE),
            &[
                ("foreground", &color.to_value()),
                ("weight", &pango::Weight::Bold.into_glib().to_value()),
            ],
        );
        tag_op
    }

    fn insert_tag(
        &mut self,
        text: &str,
        create_tag: impl Fn(&TextBuffer, bool) -> Option<TextTag>,
    ) {
        let start_offset = self.iter.offset();
        self.buf.insert(&mut self.iter, text);

        let tag_op = create_tag(&self.buf, self.is_dark);

        if let Some(tag) = tag_op {
            let start_iter = self.buf.iter_at_offset(start_offset);
            self.buf.apply_tag(&tag, &start_iter, &self.iter);
        }
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::{
        glib,
        prelude::*,
        subclass::{
            box_::BoxImpl,
            prelude::*,
            widget::{
                CompositeTemplateCallbacksClass, CompositeTemplateClass,
                CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
            },
        },
        TemplateChild,
    };

    use log::{info, warn};

    use crate::{systemd::data::UnitInfo, widget::info_window::InfoWindow};

    use super::{construct_info::fill_all_info, UnitInfoWriter};

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

            let mut info_writer = UnitInfoWriter {
                buf: buf,
                iter: start_iter,
                is_dark: is_dark,
            };

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
        }
    }
    impl WidgetImpl for UnitInfoPanelImp {}
    impl BoxImpl for UnitInfoPanelImp {}
}
