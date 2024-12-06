enum JournalAnswers {
    Tokens(Vec<colorise::Token>, String),
    Text(String),
    Markup(String),
}

use std::cell::{Cell, OnceCell, RefCell};

use gtk::{
    gio, glib,
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

use log::{debug, warn};

use crate::{
    systemd::{self, data::UnitInfo},
    widget::preferences::data::PREFERENCES,
};

use super::{colorise, rowitem::JournalEvent};

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/journal_panel.ui")]
pub struct JournalPanelImp {
    #[template_child]
    journal_refresh_button: TemplateChild<gtk::Button>,

    #[template_child]
    journal_events: TemplateChild<gtk::ListView>,

    #[template_child]
    panel_stack: TemplateChild<gtk::Stack>,

    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    unit: RefCell<Option<UnitInfo>>,

    store: OnceCell<gio::ListStore>,

    is_dark: Cell<bool>,
}

#[gtk::template_callbacks]
impl JournalPanelImp {
    #[template_callback]
    fn refresh_journal_clicked(&self, button: &gtk::Button) {
        debug!("button {:?}", button);

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
        //let journal_text: gtk::TextView = self.journal_text.clone();
        let unit = unit.clone();
        let journal_refresh_button = self.journal_refresh_button.clone();
        let oldest_first = false;
        let journal_max_events = PREFERENCES.journal_max_events();
        let panel_stack = self.panel_stack.clone();
       // let scrolled_window = self.scrolled_window.clone();
        //let journal_color: TermColor = journal_text.color().into();

        let store = self.store.get().unwrap().clone();

        glib::spawn_future_local(async move {
            let in_color = PREFERENCES.journal_colors();
            panel_stack.set_visible_child_name("spinner");
            journal_refresh_button.set_sensitive(false);  
            let journal_answer = gio::spawn_blocking(move || {
                match systemd::get_unit_journal(&unit, in_color, oldest_first, journal_max_events) {
                    Ok(journal_output) => {

               /*          store1.remove_all();
                        for line in journal_output.lines() {
                            let je = JournalEvent::new(line.to_owned());
                            store1.append(&je);
                        } */
                        let journal_answers = if in_color {
                            let tokens: Vec<colorise::Token> =
                                colorise::convert_to_tag(&journal_output);

                            JournalAnswers::Tokens(tokens, journal_output)
                        } else {
                            JournalAnswers::Text(journal_output)
                        };

                        //info!("Log size {} chars", text.len());
                        journal_answers
                    }
                    Err(error) => {
                        let text = match error.gui_description() {
                            Some(s) => s.clone(),
                            None => String::from(""),
                        };
                        JournalAnswers::Markup(text)
                    }
                }
            })
            .await
            .expect("Task needs to finish successfully.");

            match journal_answer {
                JournalAnswers::Tokens(tokens, text) => {
       /*              store.remove_all();
                    for line in text.lines() {
                        let je = JournalEvent::new(line.to_owned());
                        store.append(&je);
                    } */
                }
                JournalAnswers::Text(text) => {
    /*                 store.remove_all();
                    for line in text.lines() {
                        let je = JournalEvent::new(line.to_owned());
                        store.append(&je);
                    } */
                }
                JournalAnswers::Markup(_markup_text) => {
                    warn!("Journal error");
                }
            };

            journal_refresh_button.set_sensitive(true);

            panel_stack.set_visible_child_name("journal");
        });
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

        let list_store = gio::ListStore::new::<JournalEvent>();

        if let Err(_old) = self.store.set(list_store.clone()) {
            warn!("Store already assigned");
        }

        let selection_model = gtk::NoSelection::new(Some(list_store));

        self.journal_events.set_model(Some(&selection_model));

        let factory = gtk::SignalListItemFactory::new();
        // the "setup" stage is used for creating the widgets
        factory.connect_setup(move |_factory, item_obj| {
            let item = item_obj
                .downcast_ref::<gtk::ListItem>()
                .expect("item.downcast_ref::<gtk::ListItem>()");

            let tv = gtk::TextView::new();
            item.set_child(Some(&tv));
        });

        // the bind stage is used for "binding" the data to the created widgets on the
        // "setup" stage
        factory.connect_bind(move |_factory, item| {
            let item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("item.downcast_ref::<gtk::ListItem>()");
            // let app_info = item.item().and_downcast::<gio::AppInfo>().unwrap();

            let child = item.child().and_downcast::<gtk::TextView>().unwrap();
            let entry = item.item().and_downcast::<JournalEvent>().unwrap();

            let buf = child.buffer();
            buf.set_text(&entry.col1());
        });

        self.journal_events.set_factory(Some(&factory));
    }
}
impl WidgetImpl for JournalPanelImp {}
impl BoxImpl for JournalPanelImp {}
