use adw::subclass::prelude::ObjectSubclassIsExt;
use gettextrs::pgettext;
use glib::{
    object::{CastNone, ObjectExt},
    variant::ToVariant,
};
use gtk::{
    glib,
    prelude::{TextViewExt, WidgetExt},
};

use crate::consts::ACTION_FIND_IN_TEXT_OPEN;
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

pub fn create_menu_item(id: PanelID) -> gio::MenuItem {
    // Find in text Menu item
    let menu_label = pgettext("text_find", "Find in Text");

    let mi = gio::MenuItem::new(Some(&menu_label), None);
    mi.set_action_and_target_value(Some(ACTION_FIND_IN_TEXT_OPEN), Some(&id.to_variant()));
    mi
}

pub fn text_search_construct(
    text_view: &gtk::TextView,
    text_search_bar: &gtk::SearchBar,
    find_text_button: &gtk::ToggleButton,
    add_menu: bool,
    id: PanelID,
) {
    add_menu_fn(text_view, add_menu, id);

    let text_search_bar_content = TextSearchBar::new(text_view);

    text_search_bar.set_child(Some(&text_search_bar_content));

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
    add_menu: bool,
    id: PanelID,
) {
    add_menu_fn(text_view, add_menu, id);

    if let Some(search_bar) = text_search_bar.child().and_downcast_ref::<TextSearchBar>() {
        search_bar.imp().set_text_view(text_view);
    }
}

fn add_menu_fn(text_view: &gtk::TextView, add_menu: bool, id: PanelID) {
    if !add_menu {
        return;
    }

    let menu = gio::Menu::new();
    let item = create_menu_item(id);
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

pub enum PanelID {
    Info,
    Dependencies,
    File,
    Journal,
}

impl PanelID {
    fn to_variant(&self) -> glib::Variant {
        match self {
            PanelID::Info => 1_u8.to_variant(),
            PanelID::Dependencies => 2_u8.to_variant(),
            PanelID::File => 3_u8.to_variant(),
            PanelID::Journal => 4_u8.to_variant(),
        }
    }
}

impl From<Option<&glib::Variant>> for PanelID {
    fn from(value: Option<&glib::Variant>) -> Self {
        if let Some(variant) = value
            && let Some(val) = variant.get::<u8>()
        {
            match val {
                1 => PanelID::Info,
                2 => PanelID::Dependencies,
                3 => PanelID::File,
                4 => PanelID::Journal,
                _ => PanelID::Info,
            }
        } else {
            Self::Info
        }
    }
}

pub fn focus_on_text_entry(text_search_bar: &gtk::SearchBar) {
    if let Some(search) = text_search_bar.child().and_downcast_ref::<TextSearchBar>() {
        search.grab_focus_on_search_entry();
    }
}
