pub mod menu;
/* 
use gtk::pango::{AttrInt, AttrList, Weight};
use gtk::prelude::*;
use crate::widget::button_icon::ButtonIcon;

pub fn build_title_bar(search_bar: &gtk::SearchBar) -> TitleBar {
    // ----------------------------------------------

    let title = gtk::Label::builder()
        .label(menu::APP_TITLE)
        .single_line_mode(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .width_chars(5)
        .css_classes(["title"])
        .build();

    let header_bar = adw::HeaderBar::builder().title_widget(&title).build();

    let menu_button = menu::build_menu();

    header_bar.pack_end(&menu_button);

    let right_bar_label = gtk::Label::builder()
        .label("Service Name")
        .attributes(&{
            let attribute_list = AttrList::new();
            attribute_list.insert(AttrInt::new_weight(Weight::Bold));
            attribute_list
        })
        .build();

    let search_button = gtk::ToggleButton::new();
    search_button.set_icon_name("system-search-symbolic");
    search_button.set_tooltip_text(Some("Filter results"));
    header_bar.pack_start(&search_button);

    let refresh_button = ButtonIcon::new("Refresh", "view-refresh");
    refresh_button.set_tooltip_text(Some("Refresh results"));

    header_bar.pack_start(&refresh_button);

    header_bar.pack_start(&right_bar_label);

    search_button
        .bind_property("active", search_bar, "search-mode-enabled")
        .sync_create()
        .bidirectional()
        .build();

    TitleBar {
        header_bar,
        right_bar_label,
        search_button,
        refresh_button,
    }
}

pub fn on_startup(app: &adw::Application) {
    menu::on_startup(app);
}

pub struct TitleBar {
    pub header_bar: adw::HeaderBar,
    pub right_bar_label: gtk::Label,
    pub search_button: gtk::ToggleButton,
    pub refresh_button: ButtonIcon,
}
 */