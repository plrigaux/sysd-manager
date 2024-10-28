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
        signal_id_text: TemplateChild<adw::EntryRow>,

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

        #[template_callback]
        fn kill_signal_insert_text(&self, entry: &gtk::Entry, text: &str, position : u32) {
            info!("entry_insert_text {text}");
        }

/*         #[template_callback]
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
        } */
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


struct Signal {
    id : u32,
    name : &'static str,
    default_action : &'static str,
    comment : &'static str,
}

fn test() {

    Signal{ id : 1, name: "SIGHUP", default_action: "Terminate", comment: "Hang up controlling terminal or process" };
/* 1 SIGHUP     Terminate   Hang up controlling terminal or   Yes
    process  
2 SIGINT     Terminate   Interrupt from keyboard, Control-C    Yes
3 SIGQUIT    Dump        Quit from keyboard, Control-\         Yes
4 SIGILL     Dump        Illegal instruction                   Yes
5 SIGTRAP    Dump        Breakpoint for debugging              No
6 SIGABRT    Dump        Abnormal termination                  Yes
6 SIGIOT     Dump        Equivalent to SIGABRT                 No
7 SIGBUS     Dump        Bus error                             No
8 SIGFPE     Dump        Floating-point exception              Yes
9 SIGKILL    Terminate   Forced-process termination            Yes
10 SIGUSR1    Terminate   Available to processes               Yes
11 SIGSEGV    Dump        Invalid memory reference             Yes
12 SIGUSR2    Terminate   Available to processes               Yes
13 SIGPIPE    Terminate   Write to pipe with no readers        Yes
14 SIGALRM    Terminate   Real-timer clock                     Yes
15 SIGTERM    Terminate   Process termination                  Yes
16 SIGSTKFLT  Terminate   Coprocessor stack error              No
17 SIGCHLD    Ignore      Child process stopped or terminated  Yes
    or got a signal if traced 
18 SIGCONT    Continue    Resume execution, if stopped         Yes
19 SIGSTOP    Stop        Stop process execution, Ctrl-Z       Yes
20 SIGTSTP    Stop        Stop process issued from tty         Yes
21 SIGTTIN    Stop        Background process requires input    Yes
22 SIGTTOU    Stop        Background process requires output   Yes
23 SIGURG     Ignore      Urgent condition on socket           No
24 SIGXCPU    Dump        CPU time limit exceeded              No
25 SIGXFSZ    Dump        File size limit exceeded             No
26 SIGVTALRM  Terminate   Virtual timer clock                  No
27 SIGPROF    Terminate   Profile timer clock                  No
28 SIGWINCH   Ignore      Window resizing                      No
29 SIGIO      Terminate   I/O now possible                     No
29 SIGPOLL    Terminate   Equivalent to SIGIO                  No
30 SIGPWR     Terminate   Power supply failure                 No
31 SIGSYS     Dump        Bad system call                      No
31 SIGUNUSED  Dump        Equivalent to SIGSYS                 No */
}