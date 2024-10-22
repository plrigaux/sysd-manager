use crate::systemd::data::UnitInfo;

mod colorise;
pub mod more_colors;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

// ANCHOR: mod
glib::wrapper! {
    pub struct JournalPanel(ObjectSubclass<imp::JournalPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl JournalPanel {
    pub fn new() -> Self {
        // Create new window
        let obj: JournalPanel = glib::Object::new();

        let system_manager = adw::StyleManager::default();

        let is_dark = system_manager.is_dark();

        obj.set_dark(is_dark);

        obj
    }

    pub fn display_journal(&self, unit: &UnitInfo) {
        self.imp().display_journal(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark)
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

    use crate::{
        systemd::{self, data::UnitInfo},
        widget::{button_icon::ButtonIcon, preferences::data::PREFERENCES},
    };

    use super::{colorise, more_colors::TermColor};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/journal_panel.ui")]
    pub struct JournalPanelImp {
        #[template_child]
        journal_refresh_button: TemplateChild<gtk::Button>,

        #[template_child]
        journal_text: TemplateChild<gtk::TextView>,

        unit: RefCell<Option<UnitInfo>>,

        is_dark: Cell<bool>,
    }

    #[gtk::template_callbacks]
    impl JournalPanelImp {
        #[template_callback]
        fn refresh_journal_clicked(&self, button: &ButtonIcon) {
            info!("button {:?}", button);

            let binding = self.unit.borrow();
            let Some(unit) = binding.as_ref() else {
                warn!("no unit file");
                return;
            };

            self.update_journal(&unit)
        }

        pub(crate) fn display_journal(&self, unit: &UnitInfo) {
            let _old = self.unit.replace(Some(unit.clone()));

            self.update_journal(&unit)
        }

        /// Updates the associated journal `TextView` with the contents of the unit's journal log.
        fn update_journal(&self, unit: &UnitInfo) {
            let in_color = PREFERENCES.journal_colors();

            let text = match systemd::get_unit_journal(unit, in_color) {
                Ok(journal_output) => journal_output,
                Err(error) => {
                    let text = match error.gui_description() {
                        Some(s) => s.clone(),
                        None => String::from(""),
                    };
                    text
                }
            };

            let journal_text: &gtk::TextView = self.journal_text.as_ref();

            let buf = journal_text.buffer();
            buf.set_text(""); // clear text

            if in_color {
                let mut start_iter = buf.start_iter();
                let journal_color: TermColor = journal_text.color().into();
                let text = colorise::convert_to_mackup(&text, &journal_color);
                buf.insert_markup(&mut start_iter, &text);
            } else {
                buf.set_text(&text);
            }
        }

        pub(crate) fn set_dark(&self, is_dark: bool) {
            self.is_dark.set(is_dark);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for JournalPanelImp {
        const NAME: &'static str = "JournalPanel";
        type Type = super::JournalPanel;
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
