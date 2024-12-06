extern crate dotenv;
extern crate env_logger;
extern crate gtk;
extern crate log;

mod analyze;
mod errors;
mod systemd;
mod systemd_gui;
mod widget;

use chrono::{DateTime, Local};
use gtk::{gdk, gio, glib, prelude::*};

use log::{info, warn};

use dotenv::dotenv;
use sysd::{
    id128::Id128,
    journal::{self},
    Journal,
};
use systemd_gui::APP_ID;
use widget::{
    app_window::{menu, AppWindow},
    preferences::{data::PREFERENCES, PreferencesDialog},
};

fn main() -> glib::ExitCode {
    dotenv().ok();

    env_logger::init();

    info!("Program starting up");

    match gio::resources_register_include!("sysd-manager.gresource") {
        Ok(_) => (),
        Err(e) => warn!("Failed to register resources. Error: {:?}", e),
    }

    let _ = journal_logger();

    launch()
}

pub fn launch() -> glib::ExitCode {
    // Create a new application
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|app| {
        load_css();
        menu::on_startup(app)
    });
    app.connect_activate(build_ui);

    app.run()
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/io/github/plrigaux/sysd-manager/style.css");

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(application: &adw::Application) {
    let window = AppWindow::new(application);

    {
        let window = window.clone();
        let system_manager = adw::StyleManager::default();
        window.set_dark(system_manager.is_dark());

        system_manager.connect_dark_notify(move |a: &adw::StyleManager| {
            let is_dark = a.is_dark();
            info!("is dark {is_dark}");
            window.set_dark(is_dark);
        });
    }

    window.present();

    if PREFERENCES.is_app_first_connection() {
        info!("Is application first connection");

        let pdialog = PreferencesDialog::new();

        adw::prelude::AdwDialogExt::present(&pdialog, Some(&window));
    }
}

fn journal_logger() {
    glib::spawn_future_local(async move {
        println!("Starting journal-logger");

        gio::spawn_blocking(move || {
            println!("Preparare journal-logger");
            match journal_test() {
                Ok(_) => println!("Journal Done"),
                Err(e) => println!("Journal Error {:?}", e),
            };
        })
        .await
        .expect("Task needs to finish successfully.");
    });
}

const KEY_SYSTEMS_UNIT: &str = "_SYSTEMD_UNIT";
const KEY_UNIT2: &str = "UNIT";
const KEY_MESSAGE: &str = "MESSAGE";

const KEY_BOOT: &str = "_BOOT_ID";

const MAX_MESSAGES: usize = 1000;
fn journal_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting journal-logger");

    // Open the journal
    let mut journal = journal::OpenOptions::default()
        //.system(true)
        .open()
        .expect("Could not open journal");

    let mut i = 0;

    //journal.match_and()
    // tiny_daemon.service

    journal.match_add(KEY_SYSTEMS_UNIT, "tiny_daemon.service")?;

    let b = Id128::from_boot()?;

    println!("BOOT {}", b);

    let bt = format!("{}", b);
    journal.match_add(KEY_BOOT, bt)?;

    /* journal
    .seek(JournalSeek::ClockMonotonic {
        boot_id: b,
        usec: 0,
    })
    .expect("Could not seek "); */

    loop {
        if journal.next()? == 0 {
            println!("BREAK");
            break;
        }

        //println!("DATA {}" ,journal.display_entry_data());
        let unit_op = get_data(&mut journal, KEY_SYSTEMS_UNIT);

        let unit2_op = get_data(&mut journal, KEY_UNIT2);

        let message = get_data(&mut journal, KEY_MESSAGE);

        let boot = get_data(&mut journal, KEY_BOOT);

        let unit_name = match unit_op {
            Some(o) => o,
            None => "NONE".to_owned(),
        };

        let unit2_name = match unit2_op {
            Some(o) => o,
            None => "NONE".to_owned(),
        };

        let ts = journal.timestamp()?;

        let datetime: DateTime<Local> = ts.into();
        let date = datetime.format("%Y-%m-%d %T");

        println!("{:04} {} boot {:?}", i, date, boot);

        println!("[{}] ({}) {:?}", unit_name, unit2_name, message);

        i += 1;
        if i >= MAX_MESSAGES {
            eprintln!("done.");
            return Ok(());
        }
    }

    Ok(())
}

fn get_data(reader: &mut Journal, field: &str) -> Option<String> {
    let s = match reader.get_data(field) {
        Ok(journal_entry_op) => match journal_entry_op {
            Some(journal_entry_field) => journal_entry_field
                .value()
                .map(|v| String::from_utf8_lossy(v))
                .map(|v| v.into_owned()),
            None => None,
        },
        Err(e) => {
            println!("Error get data {:?}", e);
            None
        }
    };
    s
}
