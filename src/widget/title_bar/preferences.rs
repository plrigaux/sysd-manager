use crate::gtk::prelude::*;
use gtk::gio::Settings;
use gtk::glib;

pub fn build_preferences() -> gtk::Window {
    let gbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);

    gbox.append(&gtk::Label::new(Some("DBus level")));
    gbox.set_vexpand(false);

    let tb_system = gtk::ToggleButton::with_label("System");
    tb_system.set_vexpand(false);

    let tb_session = gtk::ToggleButton::with_label("Session");

    tb_system.set_group(Some(&tb_session));

    gbox.append(&tb_system);
    gbox.append(&tb_session);

    tb_session.set_active(true);

    let window = gtk::Window::builder()
        .title("Preferences")
        .default_height(600)
        .default_width(600)
        .child(&gbox)
        .build();

    window
}



#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "DbusLevel")]
enum DbusLevel {
    Val,
    #[enum_value(name = "Session")]
    Session,
    #[enum_value(name = "System", nick = "other")]
    System,
}