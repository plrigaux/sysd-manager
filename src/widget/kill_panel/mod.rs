use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::systemd::data::UnitInfo;

// ANCHOR: mod
glib::wrapper! {
    pub struct KillPanel(ObjectSubclass<imp::KillPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl KillPanel {
    pub fn set_unit(&self, unit: &UnitInfo) {
        self.imp().set_unit(unit);
    }

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

    use adw::{prelude::*, EnumListModel, OverlaySplitView, ToastOverlay};
    use gtk::{
        glib::{self, property::PropertySet},
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

    use log::{debug, info, warn};

    use crate::systemd::{data::UnitInfo, enums::KillWho};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/kill_panel.ui")]
    pub struct KillPanelImp {
        #[template_child]
        cancel_button: TemplateChild<gtk::Button>,

        #[template_child]
        send_button: TemplateChild<gtk::Button>,

        #[template_child]
        signal_id_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        who_to_kill: TemplateChild<adw::ComboRow>,

        #[template_child]
        unit_label: TemplateChild<gtk::Label>,

        #[template_child]
        signals_group: TemplateChild<adw::PreferencesGroup>,

        side_overlay: OnceCell<OverlaySplitView>,

        toast_overlay: OnceCell<ToastOverlay>,

        unit: RefCell<Option<UnitInfo>>,
    }

    #[gtk::template_callbacks]
    impl KillPanelImp {
        #[template_callback]
        fn button_send_clicked(&self, button: &gtk::Button) {
            info!("button_send_clicked {:?}", button);

            let text = self.signal_id_entry.text();

            let Ok(signal_id) = text.parse::<u32>() else {
                warn!("Kill signal id not a number");
                return;
            };

            let a = self.who_to_kill.selected();

            let unit_borrow = self.unit.borrow();

            let Some(unit) = unit_borrow.as_ref() else {
                warn!("No unit ");
                return;
            };

            info!("kill {} sgnal {} who {}", unit.primary(), signal_id, a)
        }

        #[template_callback]
        fn button_cancel_clicked(&self, _button: &gtk::Button) {
            info!("button_cancel_clicked");

            self.side_overlay
                .get()
                .expect("side_overlay registred")
                .set_collapsed(true);

            self.unit.set(None);
            //self.entry_text.set_text("");
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

        pub fn set_unit(&self, unit: &UnitInfo) {
            self.unit.set(Some(unit.clone()));

            let label_text = &unit.primary();
            self.unit_label.set_label(label_text);
            self.unit_label.set_tooltip_text(Some(label_text));
        }

        #[template_callback]
        fn kill_signal_text_change(&self, entry: &adw::EntryRow) {
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
    impl ObjectSubclass for KillPanelImp {
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

    impl ObjectImpl for KillPanelImp {
        fn constructed(&self) {
            self.parent_constructed();

            let model = EnumListModel::new(KillWho::static_type());

            self.who_to_kill.set_model(Some(&model));

            let expression = gtk::PropertyExpression::new(
                adw::EnumListItem::static_type(),
                None::<gtk::Expression>,
                "name",
            );

            self.who_to_kill.set_expression(Some(expression));

            let edit = self.signal_id_entry.delegate().unwrap();

            let pattern = |c: char| !c.is_ascii_digit();

            gtk::Editable::connect_insert_text(&edit, move |entry, text, position| {
                if text.contains(pattern) {
                    glib::signal::signal_stop_emission_by_name(entry, "insert-text");
                    entry.insert_text(&text.replace(pattern, ""), position);
                }
            });

            for sg in signals() {
                let action_row = adw::ActionRow::builder().title(sg.name).subtitle(sg.comment).build();

                self.signals_group.add(&action_row);

            }
        }
    }

    impl WidgetImpl for KillPanelImp {}
    impl BoxImpl for KillPanelImp {}

    struct Signal {
        id: u32,
        name: &'static str,
        default_action: &'static str,
        comment: &'static str,
    }

    fn signals() -> [Signal; 34] {
        let list = [
            Signal {
                id: 1,
                name: "SIGHUP",
                default_action: "Terminate",
                comment: "Hang up controlling terminal or process",
            },
            Signal {
                id: 2,
                name: "SIGINT",
                default_action: "Terminate",
                comment: "Interrupt from keyboard, Control-C",
            },
            Signal {
                id: 3,
                name: "SIGQUIT",
                default_action: "Dump",
                comment: "Quit from keyboard, Control-\"",
            },
            Signal {
                id: 4,
                name: "SIGILL",
                default_action: "Dump",
                comment: "Illegal instruction",
            },
            Signal {
                id: 5,
                name: "SIGTRAP",
                default_action: "Dump",
                comment: "Breakpoint for debugging",
            },
            Signal {
                id: 6,
                name: "SIGABRT",
                default_action: "Dump",
                comment: "Abnormal termination",
            },
            Signal {
                id: 6,
                name: "SIGIOT",
                default_action: "Dump",
                comment: "Equivalent to SIGABRT",
            },
            Signal {
                id: 7,
                name: "SIGBUS",
                default_action: "Dump",
                comment: "Bus error",
            },
            Signal {
                id: 8,
                name: "SIGFPE",
                default_action: "Dump",
                comment: "Floating-point exception",
            },
            Signal {
                id: 9,
                name: "SIGKILL",
                default_action: "Terminate",
                comment: "Forced-process termination",
            },
            Signal {
                id: 10,
                name: "SIGUSR1",
                default_action: "Terminate",
                comment: "Available to processes",
            },
            Signal {
                id: 11,
                name: "SIGSEGV",
                default_action: "Dump",
                comment: "Invalid memory reference",
            },
            Signal {
                id: 12,
                name: "SIGUSR2",
                default_action: "Terminate",
                comment: "Available to processes",
            },
            Signal {
                id: 13,
                name: "SIGPIPE",
                default_action: "Terminate",
                comment: "Write to pipe with no readers",
            },
            Signal {
                id: 14,
                name: "SIGALRM",
                default_action: "Terminate",
                comment: "Real-timer clock",
            },
            Signal {
                id: 15,
                name: "SIGTERM",
                default_action: "Terminate",
                comment: "Process termination",
            },
            Signal {
                id: 16,
                name: "SIGSTKFLT",
                default_action: "Terminate",
                comment: "Coprocessor stack error",
            },
            Signal {
                id: 17,
                name: "SIGCHLD",
                default_action: "Ignore",
                comment: "Child process stopped or terminated or got a signal if traced",
            },
            Signal {
                id: 18,
                name: "SIGCONT",
                default_action: "Continue",
                comment: "Resume execution, if stopped",
            },
            Signal {
                id: 19,
                name: "SIGSTOP",
                default_action: "Stop",
                comment: "Stop process execution, Ctrl-Z",
            },
            Signal {
                id: 20,
                name: "SIGTSTP",
                default_action: "Stop",
                comment: "Stop process issued from tty",
            },
            Signal {
                id: 21,
                name: "SIGTTIN",
                default_action: "Stop",
                comment: "Background process requires input",
            },
            Signal {
                id: 22,
                name: "SIGTTOU",
                default_action: "Stop",
                comment: "Background process requires output",
            },
            Signal {
                id: 23,
                name: "SIGURG",
                default_action: "Ignore",
                comment: "Urgent condition on socket",
            },
            Signal {
                id: 24,
                name: "SIGXCPU",
                default_action: "Dump",
                comment: "CPU time limit exceeded",
            },
            Signal {
                id: 25,
                name: "SIGXFSZ",
                default_action: "Dump",
                comment: "File size limit exceeded",
            },
            Signal {
                id: 26,
                name: "SIGVTALRM",
                default_action: "Terminate",
                comment: "Virtual timer clock",
            },
            Signal {
                id: 27,
                name: "SIGPROF",
                default_action: "Terminate",
                comment: "Profile timer clock",
            },
            Signal {
                id: 28,
                name: "SIGWINCH",
                default_action: "Ignore",
                comment: "Window resizing",
            },
            Signal {
                id: 29,
                name: "SIGIO",
                default_action: "Terminate",
                comment: "I/O now possible",
            },
            Signal {
                id: 29,
                name: "SIGPOLL",
                default_action: "Terminate",
                comment: "Equivalent to SIGIO",
            },
            Signal {
                id: 30,
                name: "SIGPWR",
                default_action: "Terminate",
                comment: "Power supply failure",
            },
            Signal {
                id: 31,
                name: "SIGSYS",
                default_action: "Dump",
                comment: "Bad system call",
            },
            Signal {
                id: 31,
                name: "SIGUNUSED",
                default_action: "Dump",
                comment: "Equivalent to SIGSYS",
            },
        ];

        list
    }
}
