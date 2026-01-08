use glib::WeakRef;
use gtk::{glib, prelude::*, subclass::prelude::*};

use super::TextSearchBar;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/text_find.ui")]
pub struct TextSearchBarImp {
    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,

    #[template_child]
    case_sensitive_toggle_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    regex_toggle_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    previous_match_button: TemplateChild<gtk::Button>,

    #[template_child]
    next_match_button: TemplateChild<gtk::Button>,

    text_view: WeakRef<gtk::TextView>,
}

#[gtk::template_callbacks]
impl TextSearchBarImp {
    #[template_callback]
    fn on_case_sensitive_toggled(&self, _toggle_button: &gtk::ToggleButton) {
        /*  if let Some(text_view) = self.text_view.upgrade() {
            let case_sensitive = toggle_button.is_active();
            //text_view.set_search_case_sensitive(case_sensitive);
        } */
    }

    #[template_callback]
    fn on_regex_toggled(&self, _toggle_button: &gtk::ToggleButton) {
        /*  if let Some(text_view) = self.text_view.upgrade() {
            let is_regex = toggle_button.is_active();
            //    text_view.set_search_regex(is_regex);
        } */
    }

    #[template_callback]
    fn on_previous_match_clicked(&self, _button: &gtk::Button) {
        /*  if let Some(text_view) = self.text_view.upgrade() {
            //   text_view.search_previous();
        } */
    }

    #[template_callback]
    fn on_next_match_clicked(&self, _button: &gtk::Button) {
        /*  if let Some(text_view) = self.text_view.upgrade() {
            //  text_view.search_next();
        } */
    }
}

impl TextSearchBarImp {
    pub(crate) fn set_text_view(&self, text_view: &gtk::TextView) {
        self.text_view.set(Some(text_view));
    }

    pub(crate) fn grab_focus_on_search_entry(&self) {
        self.search_entry.grab_focus();
    }
}

#[glib::object_subclass]
impl ObjectSubclass for TextSearchBarImp {
    const NAME: &'static str = "TextFind";
    type Type = TextSearchBar;
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

impl ObjectImpl for TextSearchBarImp {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for TextSearchBarImp {}
impl BoxImpl for TextSearchBarImp {}
