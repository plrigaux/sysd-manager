#[path = "../../systemd_gui.rs"]
mod systemd_gui;
use adw::HeaderBar;
use adw::prelude::AdwDialogExt;
use gtk::{Application, glib};
use gtk::{Orientation, prelude::*};

const APP_ID: &str = "org.gtk_rs.HelloWorld2";

mod flatpak;
fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

fn build_ui(app: &Application) {
    let header = HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("Test App", ""))
        .build();

    let content = gtk::Box::new(Orientation::Vertical, 0);

    let bt = gtk::Button::builder().label("Alert").build();

    content.append(&header);
    content.append(&bt);

    // Create a window and set the title
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .default_width(350)
        .content(&content)
        .build();
    {
        let win = window.clone();
        bt.connect_clicked(move |_b| {
            let dialog = flatpak::proxy_service_not_started(Some("bew test"));

            dialog.present(Some(&win));
        });
    }
    // Present window
    window.present();
}
