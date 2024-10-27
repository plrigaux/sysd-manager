use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

// ANCHOR: mod
glib::wrapper! {
    pub struct KillPanel(ObjectSubclass<imp::JournalPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl KillPanel {
    /*    pub fn display_journal(&self, unit: &UnitInfo) {
        self.imp().display_journal(unit);
    } */

    pub fn register(
        &self,
        side_overlay: &adw::OverlaySplitView,
        toast_overlay: &adw::ToastOverlay,
    ) {
        let obj = self.imp();
        obj.register(side_overlay, toast_overlay);
    }
}

mod imp {
    use std::cell::{OnceCell, RefCell};

    use adw::{OverlaySplitView, ToastOverlay};
    use gtk::{
        glib::{self, property::PropertySet},
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

    use log::{debug, info};

    use crate::systemd::data::UnitInfo;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/kill_panel.ui")]
    pub struct JournalPanelImp {
        #[template_child]
        cancel_button: TemplateChild<gtk::Button>,

        #[template_child]
        send_button: TemplateChild<gtk::Button>,

        #[template_child]
        entry_text: TemplateChild<gtk::Entry>,

        side_overlay: OnceCell<OverlaySplitView>,

        toast_overlay: OnceCell<ToastOverlay>,

        unit: RefCell<Option<UnitInfo>>,
    }

    #[gtk::template_callbacks]
    impl JournalPanelImp {
        #[template_callback]
        fn button_send_clicked(&self, button: &gtk::Button) {
            info!("button_send_clicked {:?}", button);
        }

        #[template_callback]
        fn button_cancel_clicked(&self, _button: &gtk::Button) {
            info!("button_cancel_clicked");

            self.side_overlay
                .get()
                .expect("side_overlay registred")
                .set_collapsed(true);

            self.unit.set(None);
            self.entry_text.set_text("");
        }

        pub fn register(
            &self,
            side_overlay: &adw::OverlaySplitView,
            toast_overlay: &adw::ToastOverlay,
        ) {
            self.side_overlay
                .set(side_overlay.clone())
                .expect("side_overlay once");

            self.toast_overlay
                .set(toast_overlay.clone())
                .expect("toast_overlay once");
        }

        /*         #[template_callback]
        fn entry_insert_text(&self, entry: &gtk::Entry, text: &str) {
            info!("entry_insert_text {text}");
        } */

        #[template_callback]
        fn kill_signal_text_change(&self, entry: &gtk::Entry) {
            let text = entry.text();
            debug!("entry_changed {}", text);

            if text.is_empty() {
                self.send_button.set_sensitive(false);
                return;
            }

            for c in text.chars() {
                //  if c.is
                if !c.is_digit(10) {
                    self.send_button.set_sensitive(false);
                    return;
                }
            }

            self.send_button.set_sensitive(true);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for JournalPanelImp {
        const NAME: &'static str = "KillPanel";
        type Type = super::KillPanel;
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

    impl ObjectImpl for JournalPanelImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for JournalPanelImp {}
    impl BoxImpl for JournalPanelImp {}
}
