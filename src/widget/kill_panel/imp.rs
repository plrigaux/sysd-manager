use std::cell::{Cell, OnceCell, RefCell};

use adw::{prelude::*, subclass::window::AdwWindowImpl};
use gtk::{
    TemplateChild, gio,
    glib::{self, property::PropertySet},
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
};

use log::{debug, info, warn};

use crate::{
    consts::{ERROR_CSS, WARNING_CSS},
    systemd::{self, data::UnitInfo, enums::KillWho},
    utils::writer::UnitInfoWriter,
    widget::{InterPanelAction, unit_control_panel::side_control_panel::SideControlPanel},
};

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/kill_panel.ui")]
pub struct KillPanelImp {
    #[template_child]
    send_button: TemplateChild<gtk::Button>,

    #[template_child]
    signal_id_entry: TemplateChild<adw::EntryRow>,

    #[template_child]
    who_to_kill: TemplateChild<adw::ComboRow>,

    #[template_child]
    sigqueue_value: TemplateChild<adw::EntryRow>,

    #[template_child]
    window_title: TemplateChild<adw::WindowTitle>,

    #[template_child]
    signals_box: TemplateChild<gtk::Box>,

    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    is_sigqueue: Cell<bool>,

    parent: OnceCell<SideControlPanel>,
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
        let is_sigqueue = self.is_sigqueue.get();
        let queued_signal_value = if is_sigqueue {
            match self.sigqueue_value.text().parse::<i32>() {
                Ok(v) => v,
                Err(err) => {
                    warn!("Queued Signal value not a number: {:?}", err);
                    0
                }
            }
        } else {
            0
        };

        let who: KillWho = self.who_to_kill.selected().into();

        let unit_borrow = self.unit.borrow();

        let Some(unit) = unit_borrow.as_ref() else {
            warn!("No unit ");
            return;
        };

        let unit = unit.clone();
        let button = button.clone();
        let parent = self
            .parent
            .get()
            .expect("Parent not supposed to be empty")
            .clone();
        let is_dark = self.is_dark.get();

        glib::spawn_future_local(async move {
            button.set_sensitive(false);

            let unit_ = unit.clone();
            let kill_results = gio::spawn_blocking(move || {
                if is_sigqueue {
                    systemd::queue_signal_unit(&unit_, who, signal_id, queued_signal_value)
                } else {
                    systemd::kill_unit(&unit_, who, signal_id)
                }
            })
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
                        "Kill signal {} send successfully to <span fgcolor='{blue}' font_family='monospace' size='larger'>{}</span> at <span fgcolor='{blue}'>{}</span> level",
                        signal_id,
                        unit.primary(),
                        who.as_str()
                    );

                    info!("{}", msg);

                    parent.add_toast_message(&msg, true)
                }
                Err(err) => {
                    let msg = format!(
                        "kill {} signal {} who {:?} response failed",
                        unit.primary(),
                        signal_id,
                        who
                    );
                    warn!("{msg} {:?}", err);
                    parent.add_toast_message(&msg, true)
                }
            }
        });
    }

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        let unit = match unit {
            Some(u) => u,
            None => {
                warn!("set unit to None");
                self.unit.set(None);
                self.window_title.set_subtitle("No Unit Selected");
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
        self.set_send_button_sensitivity();

        Self::validate_entry(entry, self.is_sigqueue.get(), true)
    }

    #[template_callback]
    fn sigqueue_value_changed(&self, entry: &adw::EntryRow) {
        self.set_send_button_sensitivity();

        Self::validate_entry(entry, self.is_sigqueue.get(), false)
    }

    fn validate_entry(entry: &adw::EntryRow, is_sigqueue: bool, is_signal_entry: bool) {
        let value_txt = entry.text();

        if value_txt.is_empty() {
            entry.remove_css_class(WARNING_CSS);
            entry.remove_css_class(ERROR_CSS);
        } else {
            match value_txt.parse::<i32>() {
                Ok(value) => {
                    entry.remove_css_class(ERROR_CSS);

                    if is_signal_entry && is_sigqueue {
                        if (libc::SIGRTMIN()..=libc::SIGRTMAX()).contains(&value) {
                            entry.remove_css_class(WARNING_CSS);
                        } else {
                            entry.add_css_class(WARNING_CSS);
                        }
                    }
                }
                Err(parse_int_error) => match parse_int_error.kind() {
                    std::num::IntErrorKind::PosOverflow | std::num::IntErrorKind::NegOverflow => {
                        entry.remove_css_class(ERROR_CSS);
                        entry.add_css_class(WARNING_CSS);
                    }

                    _ => {
                        entry.add_css_class(ERROR_CSS);
                        entry.remove_css_class(WARNING_CSS);
                    }
                },
            }
        }
    }
    #[template_callback]
    fn who_to_kill_activate(&self, combo_row: &adw::ComboRow) {
        debug!("who_to_kill_activate {}", combo_row.index());
    }

    #[template_callback]
    fn who_to_kill_activated(&self, combo_row: &adw::ComboRow) {
        debug!("who_to_kill_activated {}", combo_row.index());
    }

    fn contruct_signals_description(&self, sg: Signal) {
        let title = sg.name;
        let action_row = adw::ActionRow::builder()
            .title(title)
            .subtitle(format!(
                "{}\nDefault Action: {}",
                sg.comment, sg.default_action
            ))
            .subtitle_lines(2)
            .margin_end(5)
            .margin_start(5)
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
        self.signals_box.append(&action_row);
    }
}

impl KillPanelImp {
    pub(super) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);
    }

    pub(super) fn set_inter_action(&self, action: &InterPanelAction) {
        match *action {
            InterPanelAction::IsDark(is_dark) => self.set_dark(is_dark),
            InterPanelAction::UnitChange(unit) => self.set_unit(unit),
            _ => (),
        }
    }

    pub(super) fn set_is_signal(&self, is_signal: bool) {
        self.is_sigqueue.set(is_signal);

        if is_signal {
            self.window_title
                .set_title("Queue a Realtime Signal to Unit");
            self.sigqueue_value.set_visible(true);

            let min = libc::SIGRTMIN();
            let max = libc::SIGRTMAX();
            let span = max - min;
            let span_d2 = span / 2;

            for id in min..=max {
                let offset = id - min;

                let name = if offset == 0 {
                    "SIGRTMIN".to_string()
                } else if offset == span {
                    "SIGRTMAX".to_string()
                } else if offset > span_d2 {
                    format!("SIGRTMAX-{}", span - offset)
                } else {
                    format!("SIGRTMIN+{offset}")
                };

                let signal = Signal {
                    id: (id as u32),
                    name: &name,
                    default_action: "Terminate",
                    comment: "Real-time signal",
                };
                self.contruct_signals_description(signal);
            }
        } else {
            self.window_title.set_title("Send a Kill Signal to Unit");
            self.sigqueue_value.set_visible(false);
            for signal in signals() {
                self.contruct_signals_description(signal);
            }
        }
    }

    fn set_send_button_sensitivity(&self) {
        let text = self.signal_id_entry.text();

        let button_sensitive = match (
            text.is_empty(),
            text.contains(pattern_not_digit),
            self.unit.borrow().is_some(),
            self.is_sigqueue.get(),
        ) {
            (false, false, true, false) => true,
            (false, false, true, true) => {
                let signal_value_text = self.sigqueue_value.text();
                matches!(
                    (
                        signal_value_text.is_empty(),
                        signal_value_text.contains(pattern_not_digit)
                    ),
                    (false, false)
                )
            }
            _a => {
                debug!("a {:?}", _a);
                false
            }
        };

        self.send_button.set_sensitive(button_sensitive);
    }

    pub(crate) fn set_parent(&self, parent: &SideControlPanel) {
        self.parent
            .set(parent.clone())
            .expect("parent should be set once");
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

        self.who_to_kill
            .connect_selected_item_notify(|who_to_kill| {
                let o = who_to_kill.selected_item();
                let Some(object) = o else {
                    return;
                };

                let item = object
                    .downcast_ref::<adw::EnumListItem>()
                    .expect("Suppose to be a EnumListItem");

                let kill_who: KillWho = item.value().into();

                who_to_kill.set_subtitle(kill_who.description());
            });

        let edit = self.signal_id_entry.delegate().unwrap();

        gtk::Editable::connect_insert_text(&edit, move |entry, text, position| {
            if text.contains(pattern_not_digit) {
                glib::signal::signal_stop_emission_by_name(entry, "insert-text");
                entry.insert_text(&text.replace(pattern_not_digit, ""), position);
            }
        });
    }
}

impl WidgetImpl for KillPanelImp {}
impl WindowImpl for KillPanelImp {
    fn close_request(&self) -> glib::Propagation {
        self.parent_close_request();
        self.unit.set(None);
        if let Some(parent) = self.parent.get() {
            parent.unlink_child(self.is_sigqueue.get());
        }
        glib::Propagation::Proceed
    }
}
impl AdwWindowImpl for KillPanelImp {}

fn pattern_not_digit(c: char) -> bool {
    !c.is_ascii_digit()
}

//#[allow(dead_code)]
struct Signal<'a> {
    id: u32,
    name: &'a str,
    default_action: &'static str,
    comment: &'static str,
}

fn signals<'a>() -> [Signal<'a>; 34] {
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
