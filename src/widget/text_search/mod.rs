use adw::subclass::prelude::ObjectSubclassIsExt;
use gettextrs::pgettext;
use glib::object::{CastNone, ObjectExt};
use gtk::{
    glib,
    prelude::{TextViewExt, WidgetExt},
};

use crate::widget::app_window::AppWindow;
mod imp;

glib::wrapper! {
    pub struct TextSearchBar(ObjectSubclass<imp::TextSearchBarImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl TextSearchBar {
    pub fn new(text_view: &gtk::TextView) -> TextSearchBar {
        let obj: TextSearchBar = glib::Object::new();

        obj.imp().set_text_view(text_view);

        obj
    }

    pub fn grab_focus_on_search_entry(&self) {
        self.imp().grab_focus_on_search_entry();
    }

    pub fn clear_tags(&self) {
        self.imp().clear_tags();
    }

    pub fn find_text(&self) {
        self.imp().new_find_in_text();
    }

    pub fn new_added_text(
        &self,
        buff: &gtk::TextBuffer,
        start_iter: gtk::TextIter,
        end_iter: gtk::TextIter,
    ) {
        self.imp().new_added_text(buff, start_iter, end_iter);
    }
}

pub fn create_menu_item(
    action_name_base: &str,
    _text_search_bar: &gtk::SearchBar,
) -> gio::MenuItem {
    // Find in text Menu item
    let menu_label = pgettext("text_find", "Open Find in Text");

    let mut action_name = String::from("win.");
    action_name.push_str(action_name_base);

    //mi.set_action_and_target_value(
    //    Some(&action_name),
    //Some(&text_search_bar.is_search_mode().to_variant()),
    //);
    gio::MenuItem::new(Some(&menu_label), Some(&action_name))
}

pub fn text_search_construct(
    text_view: &gtk::TextView,
    text_search_bar: &gtk::SearchBar,
    find_text_button: &gtk::ToggleButton,
    action_name_base: &str,
    add_menu: bool,
) {
    add_menu_fn(action_name_base, text_view, text_search_bar, add_menu);

    let text_search_bar_content = TextSearchBar::new(text_view);

    text_search_bar.set_child(Some(&text_search_bar_content));

    //find_text_button.set_action_target(Some(&format!("win.{action_name_base}")));
    //find_text_button.set_action_target_value(Some(&true.to_variant()));
    find_text_button
        .bind_property("active", text_search_bar, "search-mode-enabled")
        .bidirectional()
        .build();

    //toggle button tooltip text
    let tooltip_text = pgettext("text_find", "Open Find in Text");
    find_text_button.set_tooltip_text(Some(&tooltip_text));

    text_search_bar.connect_search_mode_enabled_notify(|search_bar| {
        if let Some(text_search_bar) = search_bar.child().and_downcast_ref::<TextSearchBar>() {
            if search_bar.is_search_mode() {
                text_search_bar.find_text();
            } else {
                text_search_bar.clear_tags();
            }
        };
    });
}

pub fn on_new_text(search_bar: &gtk::SearchBar) {
    if !search_bar.is_search_mode() {
        return;
    }

    if let Some(text_search_bar) = search_bar.child().and_downcast_ref::<TextSearchBar>() {
        text_search_bar.find_text();
    }
}

pub fn update_text_view(
    text_search_bar: &gtk::SearchBar,
    text_view: &gtk::TextView,
    action_name_base: &str,
    add_menu: bool,
) {
    add_menu_fn(action_name_base, text_view, text_search_bar, add_menu);

    if let Some(search_bar) = text_search_bar.child().and_downcast_ref::<TextSearchBar>() {
        search_bar.imp().set_text_view(text_view);
    }
}

fn add_menu_fn(
    action_name_base: &str,
    text_view: &gtk::TextView,
    text_search_bar: &gtk::SearchBar,
    add_menu: bool,
) {
    if !add_menu {
        return;
    }

    let menu = gio::Menu::new();
    let item = create_menu_item(action_name_base, text_search_bar);
    menu.append_item(&item);

    let menu_sec = gio::Menu::new();

    menu_sec.append_section(None, &menu);
    text_view.set_extra_menu(Some(&menu_sec));
}

pub fn new_added_text(
    text_search_bar: &gtk::SearchBar,
    buff: &gtk::TextBuffer,
    start_iter: gtk::TextIter,
    end_iter: gtk::TextIter,
) {
    if let Some(search_bar) = text_search_bar.child().and_downcast_ref::<TextSearchBar>() {
        search_bar.imp().new_added_text(buff, start_iter, end_iter);
    }
}

pub fn create_action_entry(
    text_search_bar: &gtk::SearchBar,
    action_name: &str,
) -> gio::ActionEntry<AppWindow> {
    let text_search_bar = text_search_bar.clone();
    //let is_search_mode = !text_search_bar.is_search_mode();
    let text_search_bar_action_entry: gio::ActionEntry<AppWindow> =
        gio::ActionEntry::builder(action_name)
            .activate(move |_app_window: &AppWindow, _simple_action, _variant| {
                //if let Some(state) = simple_action.state()
                //    && let Some(state) = state.get::<bool>()
                //{
                text_search_bar.set_search_mode(true);
                if let Some(search) = text_search_bar.child().and_downcast_ref::<TextSearchBar>() {
                    search.grab_focus_on_search_entry();
                }
                //let state = !state;
                //simple_action.set_state(&state.to_variant());
                //}
            })
            //.state(is_search_mode.to_variant())
            //.parameter_type(Some(glib::VariantTy::BOOLEAN))
            .build();

    text_search_bar_action_entry
}
