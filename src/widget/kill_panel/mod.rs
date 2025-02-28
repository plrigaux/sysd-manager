use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use crate::systemd::data::UnitInfo;

use super::InterPanelAction;

// ANCHOR: mod
glib::wrapper! {
    pub struct KillPanel(ObjectSubclass<imp::KillPanelImp>)
        @extends adw::Window, gtk::Window, gtk::Widget,
        //@implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;

        @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
        gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl KillPanel {
    pub fn new(unit: Option<&UnitInfo>, is_dark: bool) -> Self {
        let obj: KillPanel = glib::Object::new();
        obj.set_unit(unit);
        obj.set_inter_action(&InterPanelAction::IsDark(is_dark));
        obj
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        self.imp().set_unit(unit);
    }

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        self.imp().set_inter_action(action);
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
    use std::cell::{Cell, OnceCell, RefCell};

    use adw::{prelude::*, subclass::window::AdwWindowImpl};
    use gtk::{
        gio,
        glib::{self, property::PropertySet},
        subclass::{
            prelude::*,
            widget::{
                CompositeTemplateCallbacksClass, CompositeTemplateClass,
                CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
            },
        },
        TemplateChild,
    };

    use log::{debug, info, warn};

    use crate::{
        systemd::{self, data::UnitInfo, enums::KillWho},
        utils::writer::UnitInfoWriter,
        widget::InterPanelAction,
    };

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
        who_to_kill: TemplateChild<gtk::DropDown>,

        #[template_child]
        window_title: TemplateChild<adw::WindowTitle>,

        #[template_child]
        signals_group: TemplateChild<adw::PreferencesGroup>,

        /*  #[template_child]
        signals_group_box: TemplateChild<gtk::Box>, */
        //side_overlay: OnceCell<adw::OverlaySplitView>,
        toast_overlay: OnceCell<adw::ToastOverlay>,

        unit: RefCell<Option<UnitInfo>>,

        is_dark: Cell<bool>,
    }

    #[gtk::template_callbacks]
    impl KillPanelImp {
        #[template_callback]
        fn button_send_clicked(&self, button: &gtk::Button) {
            info!("button_send_clicked {:?}", button);

            let text = self.signal_id_entry.text();

            let Ok(signal_id) = text.parse::<i32>() else {
                warn!("Kill signal id not a number");
                return;
            };

            let who: KillWho = self.who_to_kill.selected().into();

            let unit_borrow = self.unit.borrow();

            let Some(unit) = unit_borrow.as_ref() else {
                warn!("No unit ");
                return;
            };

            let unit = unit.clone();
            let button = button.clone();
            let toast_overlay = self
                .toast_overlay
                .get()
                .expect("not supposed to be empty")
                .clone();
            let is_dark = self.is_dark.get();
            glib::spawn_future_local(async move {
                button.set_sensitive(false);

                let unit_ = unit.clone();
                let kill_results =
                    gio::spawn_blocking(move || systemd::kill_unit(&unit_, who, signal_id))
                        .await
                        .expect("Task kill_unit needs to finish successfully.");

                button.set_sensitive(true);

                match kill_results {
                    Ok(_) => {
                        let blue = if is_dark {
                            UnitInfoWriter::blue_dark()
                        } else {
                            UnitInfoWriter::blue_light()
                        };

                        let msg = format!(
                            "Kill signal {} send succesfully to <span fgcolor='{blue}' font_family='monospace' size='larger'>{}</span> at <span fgcolor='{blue}'>{}</span> level",
                            signal_id,
                            unit.primary(),
                            who.as_str()
                        );

                        info!("{}", msg);

                        let toast = adw::Toast::builder().title(&msg).use_markup(true).build();
                        toast_overlay.add_toast(toast)
                    }
                    Err(e) => {
                        let msg = format!(
                            "kill {} signal {} who {:?} response {:?}",
                            unit.primary(),
                            signal_id,
                            who,
                            e
                        );
                        warn!("{msg}");
                        let toast = adw::Toast::new(&msg);
                        toast_overlay.add_toast(toast)
                    }
                }
            });
        }

        #[template_callback]
        fn button_cancel_clicked(&self, _button: &gtk::Button) {
            info!("button_cancel_clicked");

            self.unit.set(None);
            //self.entry_text.set_text("");
        }

        pub fn register(
            &self,
            _side_overlay: &adw::OverlaySplitView,
            toast_overlay: &adw::ToastOverlay,
        ) {
            self.toast_overlay
                .set(toast_overlay.clone())
                .expect("toast_overlay once");
        }

        pub fn set_unit(&self, unit: Option<&UnitInfo>) {
            let unit = match unit {
                Some(u) => u,
                None => {
                    self.unit.set(None);
                    return;
                }
            };

            self.unit.set(Some(unit.clone()));

            let label_text = &unit.primary();

            self.window_title.set_subtitle(label_text);

            self.set_send_button_sensitivity();
        }

        #[template_callback]
        fn kill_signal_text_change(&self, entry: &adw::EntryRow) {
            let text = entry.text();
            debug!("entry_changed {}", text);

            self.set_send_button_sensitivity();
        }

        pub(crate) fn set_dark(&self, is_dark: bool) {
            self.is_dark.set(is_dark);
        }

        pub(crate) fn set_inter_action(&self, action: &InterPanelAction) {
            if let InterPanelAction::IsDark(is_dark) = *action {
                self.set_dark(is_dark)
            }
        }

        fn set_send_button_sensitivity(&self) {
            let text = self.signal_id_entry.text();

            match (
                text.is_empty(),
                text.contains(pattern_not_digit),
                self.unit.borrow().is_some(),
            ) {
                (false, false, true) => self.send_button.set_sensitive(true),
                _ => self.send_button.set_sensitive(false),
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for KillPanelImp {
        const NAME: &'static str = "KillPanel";
        type Type = super::KillPanel;
        type ParentType = adw::Window;

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

            let expression = gtk::PropertyExpression::new(
                adw::EnumListItem::static_type(),
                None::<gtk::Expression>,
                "nick",
            );

            self.who_to_kill.set_expression(Some(expression));

            let model = adw::EnumListModel::new(KillWho::static_type());

            self.who_to_kill.set_model(Some(&model));

            let edit = self.signal_id_entry.delegate().unwrap();

            gtk::Editable::connect_insert_text(&edit, move |entry, text, position| {
                if text.contains(pattern_not_digit) {
                    glib::signal::signal_stop_emission_by_name(entry, "insert-text");
                    entry.insert_text(&text.replace(pattern_not_digit, ""), position);
                }
            });

            for sg in signals() {
                let title = sg.name;
                let action_row = adw::ActionRow::builder()
                    .title(title)
                    .subtitle(sg.comment)
                    .build();

                let button_label = sg.id.to_string();
                let action_button = gtk::Button::builder()
                    .label(&button_label)
                    .css_classes(["circular", "raised"])
                    .valign(gtk::Align::BaselineCenter)
                    .build();

                let entry_row = self.signal_id_entry.clone();
                action_button.connect_clicked(move |_| {
                    entry_row.set_text(&button_label);
                });
                action_row.add_suffix(&action_button);
                self.signals_group.add(&action_row);
            }
        }
    }

    impl WidgetImpl for KillPanelImp {}
    impl WindowImpl for KillPanelImp {
        /*         fn close_request(&self) -> glib::Propagation {
            println!("{:?}", self.obj().default_size());

            glib::Propagation::Proceed
        } */
    }
    impl AdwWindowImpl for KillPanelImp {}

    fn pattern_not_digit(c: char) -> bool {
        !c.is_ascii_digit()
    }

    #[allow(dead_code)]
    struct Signal {
        id: u32,
        name: &'static str,
        default_action: &'static str,
        comment: &'static str,
    }

    fn signals() -> [Signal; 34] {
        [
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
        ]
    }
}
