use log::{debug, warn};

use crate::{gtk::subclass::prelude::*, widget::unit_list::UnitListPanel};
use gtk::prelude::*;

glib::wrapper! {
    pub struct UnitPopMenu(ObjectSubclass<imp::UnitPopMenuImp>)
        @extends gtk::Popover,  gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable, gtk::Native,gtk::ShortcutManager;
}

impl UnitPopMenu {
    pub fn new(
        units_browser: &gtk::ColumnView,
        unit_list_panel: &UnitListPanel,
        filtered_list: &gtk::FilterListModel,
    ) -> Self {
        let obj: UnitPopMenu = glib::Object::new();
        obj.imp()
            .set_gesture(units_browser, unit_list_panel, filtered_list);
        obj
    }

    fn refresh_buttons_style(&self) {
        self.imp().refresh_buttons_style();
    }
}

mod imp {
    use std::{
        cell::{OnceCell, RefCell},
        rc::Rc,
    };

    use gettextrs::pgettext;
    use gtk::{gdk, glib::subclass::types::ObjectSubclass, prelude::*, subclass::prelude::*};
    use log::{debug, info, warn};

    use crate::{
        consts::{DESTRUCTIVE_ACTION, FLAT, SUGGESTED_ACTION},
        format2,
        systemd::{data::UnitInfo, enums::EnablementStatus},
        upgrade,
        utils::palette::blue,
        widget::{InterPanelMessage, unit_list::UnitListPanel},
    };

    macro_rules! unit {
        ($self:expr) => {{
            let borrow = $self.unit.borrow();
            let Some(unit) = borrow.as_ref() else {
                warn!("Pop menu has no unit");
                return;
            };
            unit.clone()
        }};
    }

    macro_rules! unit_list_panel {
        ($self:expr) => {{
            let Some(unit_list_panel) = $self.unit_list_panel.get() else {
                warn!("Pop menu has Unit_list_panel");
                return;
            };
            unit_list_panel
        }};
    }

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_pop_menu.ui")]
    pub struct UnitPopMenuImp {
        #[template_child]
        unit_label: TemplateChild<gtk::Label>,

        #[template_child]
        start_button: TemplateChild<gtk::Button>,

        #[template_child]
        stop_button: TemplateChild<gtk::Button>,

        #[template_child]
        restart_button: TemplateChild<gtk::Button>,

        #[template_child]
        enable_button: TemplateChild<gtk::Button>,

        #[template_child]
        disable_button: TemplateChild<gtk::Button>,

        #[template_child]
        reenable_button: TemplateChild<gtk::Button>,

        #[template_child]
        relaod_button: TemplateChild<gtk::Button>,

        pub(super) units_browser: OnceCell<gtk::ColumnView>,
        pub(super) unit_list_panel: OnceCell<UnitListPanel>,
        unit: RefCell<Option<UnitInfo>>,
    }

    #[gtk::template_callbacks]
    impl UnitPopMenuImp {
        #[template_callback]
        fn start_button_clicked(&self, button: gtk::Button) {
            let unit = unit!(self);
            let pop_menu = self.obj().clone();
            let inter_message = InterPanelMessage::StartUnit(
                button,
                unit,
                Rc::new(Box::new(move || pop_menu.refresh_buttons_style())),
            );

            unit_list_panel!(self).button_action(&inter_message);
        }

        #[template_callback]
        fn stop_button_clicked(&self, button: gtk::Button) {
            let unit = unit!(self);
            let pop_menu = self.obj().clone();
            let inter_message = InterPanelMessage::StopUnit(
                button,
                unit,
                Rc::new(Box::new(move || pop_menu.refresh_buttons_style())),
            );

            unit_list_panel!(self).button_action(&inter_message);
        }

        #[template_callback]
        fn restart_button_clicked(&self, button: gtk::Button) {
            let unit = unit!(self);
            let pop_menu = self.obj().clone();
            let inter_message = InterPanelMessage::ReStartUnit(
                button,
                unit,
                Rc::new(Box::new(move || pop_menu.refresh_buttons_style())),
            );

            unit_list_panel!(self).button_action(&inter_message);
        }

        #[template_callback]
        fn enable_button_clicked(&self, _button: gtk::Button) {
            let unit = unit!(self);
            let pop_menu = self.obj().clone();
            let inter_message = InterPanelMessage::EnableUnit(
                unit,
                Rc::new(Box::new(move || pop_menu.refresh_buttons_style())),
            );

            unit_list_panel!(self).button_action(&inter_message);
        }

        #[template_callback]
        fn disable_button_clicked(&self, _button: gtk::Button) {
            let unit = unit!(self);
            let pop_menu = self.obj().clone();
            let inter_message = InterPanelMessage::DisableUnit(
                unit,
                Rc::new(Box::new(move || pop_menu.refresh_buttons_style())),
            );

            unit_list_panel!(self).button_action(&inter_message);
        }

        #[template_callback]
        fn reenable_button_clicked(&self, _button: gtk::Button) {
            let unit = unit!(self);
            let pop_menu = self.obj().clone();
            let inter_message = InterPanelMessage::ReenableUnit(
                unit,
                Rc::new(Box::new(move || pop_menu.refresh_buttons_style())),
            );

            unit_list_panel!(self).button_action(&inter_message);
        }

        #[template_callback]
        fn reload_button_clicked(&self, button: gtk::Button) {
            let unit = unit!(self);
            let pop_menu = self.obj().clone();
            let inter_message = InterPanelMessage::ReloadUnit(
                button,
                unit,
                Rc::new(Box::new(move || pop_menu.refresh_buttons_style())),
            );

            unit_list_panel!(self).button_action(&inter_message);
        }

        fn set_unit(&self, unit: Option<&UnitInfo>) {
            if let Some(unit) = unit {
                self.unit.replace(Some(unit.clone()));

                let primary_name = unit.primary();
                self.unit_label.set_label(&primary_name);
                self.unit_label.set_tooltip_text(Some(&primary_name));

                let is_dark = self.unit_list_panel.get().expect("Sould Be Set").is_dark();
                let blue = blue(is_dark).get_color();

                self.set_tooltip(
                    &self.start_button,
                    blue,
                    &primary_name,
                    &pgettext("controls", "Start unit {}"),
                );

                self.set_tooltip(
                    &self.stop_button,
                    blue,
                    &primary_name,
                    &pgettext("controls", "Stop unit {}"),
                );

                self.set_tooltip(
                    &self.restart_button,
                    blue,
                    &primary_name,
                    &pgettext("controls", "Restart unit {}"),
                );

                self.set_tooltip(
                    &self.enable_button,
                    blue,
                    &primary_name,
                    &pgettext("controls", "Enable unit {}"),
                );

                self.set_tooltip(
                    &self.disable_button,
                    blue,
                    &primary_name,
                    &pgettext("controls", "Disable unit {}"),
                );

                self.set_tooltip(
                    &self.reenable_button,
                    blue,
                    &primary_name,
                    &pgettext("controls", "Disable and then Enable unit {}"),
                );

                self.set_tooltip(
                    &self.relaod_button,
                    blue,
                    &primary_name,
                    &pgettext("controls", "Reload unit {} configuration by calling the <b>ExecReload</b> unit file instruction"),
                );

                self.set_buttons_style(unit);
            } else {
                self.unit.replace(None);
            }
        }

        fn set_tooltip(&self, button: &gtk::Button, blue: &str, unit_primary: &str, tooltip: &str) {
            let unit_str = format!(
                "<span fgcolor='{}' font_family='monospace' size='larger' weight='bold'>{}</span>",
                blue, unit_primary
            );

            let tooltip = format2!(tooltip, unit_str);

            button.set_tooltip_markup(Some(&tooltip));
        }

        pub(super) fn set_gesture(
            &self,
            units_browser: &gtk::ColumnView,
            unit_list_panel: &UnitListPanel,
            filtered_list: &gtk::FilterListModel,
        ) {
            self.obj().set_parent(units_browser);

            let _ = self.units_browser.set(units_browser.clone());
            let _ = self.unit_list_panel.set(unit_list_panel.clone());

            let gesture = gtk::GestureClick::builder()
                .button(gtk::gdk::BUTTON_SECONDARY)
                .build();

            let units_browser_wr = units_browser.downgrade();
            let filtered_list = filtered_list.downgrade();

            let pop_up = self.obj().downgrade();

            gesture.connect_pressed(move |_gesture_click, n_press, x, y| {
                debug!("Pressed n_press: {n_press} x: {x} y: {y}");

                let units_browser = upgrade!(units_browser_wr);
                let filtered_list = upgrade!(filtered_list);

                let pop_up = upgrade!(pop_up);

                let Some(adjustement_offset) = units_browser
                    .vadjustment()
                    .map(|adjustment| adjustment.value())
                else {
                    warn!("Failed to retreive the adjusment heigth");
                    return;
                };

                let adjusted_y = (adjustement_offset + y) as i32;

                let mut child_op = units_browser.first_child();

                let mut header_height = 0;

                let mut line_id = -2;
                while let Some(ref child) = child_op {
                    let child_name = child.type_().name();
                    if child_name == "GtkColumnViewRowWidget" {
                        header_height = child.height();
                    } else if child_name == "GtkColumnListView" {
                        line_id = super::retreive_row_id(child, adjusted_y, header_height);
                        break;
                    }

                    child_op = child.next_sibling();
                }

                debug!("Line id {line_id} list count {}", filtered_list.n_items());

                if line_id < 0 {
                    warn!("Some wrongs line_no {line_id}");
                    return;
                }

                let line_id = line_id as u32;
                if let Some(object) = filtered_list.item(line_id) {
                    let unit = object.downcast_ref::<UnitInfo>().expect("Ok");
                    info!(
                        "Pointing on Unit {} state {}",
                        unit.primary(),
                        unit.active_state()
                    );
                    pop_up.imp().set_unit(Some(unit));
                    pop_up.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                    pop_up.popup();
                } else if line_id >= filtered_list.n_items() {
                    warn!(
                        "Line id: {line_id} is over or equals the number of lines {}",
                        filtered_list.n_items()
                    );
                } else {
                    warn!("Menu right click. Something wrong");
                }
            });
            gesture.connect_released(|_g, n_press, x, y| {
                debug!("Released n_press: {n_press} x: {x} y: {y}")
            });

            units_browser.add_controller(gesture);
        }

        pub(super) fn refresh_buttons_style(&self) {
            let Some(ref unit) = *self.unit.borrow() else {
                warn!("Pop menu has no unit");
                return;
            };

            self.set_buttons_style(unit);
        }

        fn set_buttons_style(&self, unit: &UnitInfo) {
            if unit.active_state().is_inactive() {
                self.start_button.remove_css_class(FLAT);
                self.start_button.add_css_class(SUGGESTED_ACTION);
                self.stop_button.remove_css_class(DESTRUCTIVE_ACTION);
            } else {
                self.start_button.add_css_class(FLAT);
                self.start_button.remove_css_class(SUGGESTED_ACTION);
                self.stop_button.add_css_class(DESTRUCTIVE_ACTION);
            }

            if matches!(
                unit.enable_status(),
                EnablementStatus::Disabled | EnablementStatus::Masked
            ) {
                self.enable_button.set_sensitive(true);
                self.reenable_button.set_sensitive(false);
                self.disable_button.set_sensitive(false);
            } else {
                self.enable_button.set_sensitive(false);
                self.reenable_button.set_sensitive(true);
                self.disable_button.set_sensitive(true);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnitPopMenuImp {
        const NAME: &'static str = "UnitPopMenu";
        type Type = super::UnitPopMenu;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnitPopMenuImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for UnitPopMenuImp {}
    //impl ShortcutManagerImpl for UnitPopMenuImp {}
    impl PopoverImpl for UnitPopMenuImp {}
}

/// This works, because, we assume that each line have same height
fn retreive_row_id(widget: &gtk::Widget, y: i32, header_height: i32) -> i32 {
    debug!("widjet {widget:?}");

    let mut child_op = widget.first_child();

    debug!("child_op {child_op:?}");

    let mut row_height = -1;
    while let Some(child) = child_op {
        let w_type_name = child.type_().name();
        debug!("widget type name: {w_type_name}");
        if w_type_name == "GtkColumnViewRowWidget" {
            row_height = child.height();
            if row_height > 0 {
                break;
            }
        }

        child_op = child.next_sibling();
    }

    if row_height > 0 {
        debug!("y {y} header_height {header_height} row_height {row_height}");
        (y - header_height) / row_height
    } else {
        warn!("Not a valid row height {row_height}");
        -1
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_derive_row_id() {
        let y = 44;
        let header_height = 10;
        let row_height = 20;

        let row = (y - header_height) / row_height;

        println!("{row}");

        let y = 20;

        let row = (y - header_height) / row_height;

        println!("{row}");
    }
}
