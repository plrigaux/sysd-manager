use adw::prelude::NavigationPageExt;
use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};

use crate::widget::creator::{PageType, SaveUnit, UnitCreatorWindow};

glib::wrapper! {

    pub struct LaunchCreatorPage(ObjectSubclass<imp::LaunchCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl LaunchCreatorPage {
    pub fn new(window: WeakRef<UnitCreatorWindow>, page: PageType) -> Self {
        let obj: LaunchCreatorPage = glib::Object::new();
        obj.imp().set_window(window);
        obj.set_tag(Some(page.id()));
        obj
    }

    pub fn update_page(&self) {
        self.imp().update_page();
    }

    pub fn handle_create_after(&self, msg: SaveUnit) {
        self.imp().handle_create_after(msg)
    }
}

mod imp {

    use super::*;
    use crate::{
        format2, upgrade, upgrade_opt,
        widget::creator::{UnitCreateType, imp::UnitCreatorWindowImp},
    };
    use adw::{prelude::ActionRowExt, subclass::prelude::*};
    use base::enums::UnitDBusLevel;
    use gettextrs::gettext;
    use gtk::{glib, prelude::*};
    use std::cell::{Cell, OnceCell};
    use tracing::{error, info, warn};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/launch_creator_page.ui")]
    #[properties(wrapper_type = super::LaunchCreatorPage)]
    pub struct LaunchCreatorPageImp {
        #[property(get, set, default)]
        creation_type: Cell<UnitCreateType>,

        #[template_child]
        daemon_reload_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        enable_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        start_switch: TemplateChild<adw::SwitchRow>,

        #[template_child]
        service_file_action: TemplateChild<adw::ActionRow>,
        #[template_child]
        timer_file_action: TemplateChild<adw::ActionRow>,
        #[template_child]
        service_file_button: TemplateChild<gtk::Button>,
        #[template_child]
        timer_file_button: TemplateChild<gtk::Button>,

        pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,
        // #[property(get)]
        // pub(super) data: OnceCell<UnitFileData>,
    }

    impl LaunchCreatorPageImp {
        pub fn set_window(&self, window_weak: WeakRef<UnitCreatorWindow>) {
            let _ = self.window.set(window_weak.clone());

            let window = upgrade!(window_weak);

            const ACTION_CREATOR_DAEMON_RELOAD: &str = "creator.daemon-reload";
            let daemon_reload_entry: gio::ActionEntry<_> = {
                let enable_switch = self.enable_switch.downgrade();
                let start_switch = self.start_switch.downgrade();
                gio::ActionEntry::builder(&ACTION_CREATOR_DAEMON_RELOAD[8..])
                    .activate(move |_, action, _| {
                        let Some(state) = action.state().and_then(|var| var.get::<bool>()) else {
                            return;
                        };
                        let state = !state;
                        action.set_state(&(state).to_variant());

                        if let Some(enable_switch) = enable_switch.upgrade() {
                            enable_switch.set_sensitive(state);
                            enable_switch.set_active(false);
                        }

                        if let Some(start_switch) = start_switch.upgrade() {
                            start_switch.set_sensitive(state);
                            start_switch.set_active(false);
                        }
                    })
                    .parameter_type(Some(glib::VariantTy::BOOLEAN))
                    .state(false.to_variant())
                    .build()
            };

            let action_group = window.action_group();
            action_group.add_action_entries([daemon_reload_entry]);
        }

        pub(crate) fn handle_create_after(&self, msg: SaveUnit) {
            if !matches!(msg, SaveUnit::Created) {
                return;
            }

            self.service_file_button.set_sensitive(true);
            self.timer_file_button.set_sensitive(true);

            if self.daemon_reload_switch.is_active() {
                let window = upgrade_opt!(self.window.get());
                let level = window.level();
                info!("Call reload deamon");

                self.daemon_reload(window, level);
            }
        }

        fn daemon_reload(&self, window: UnitCreatorWindow, dbus_level: UnitDBusLevel) {
            let page = self.obj().clone();
            glib::spawn_future_local(async move {
                // simple_action.set_enabled(false);

                let (sender, receiver) = tokio::sync::oneshot::channel();
                systemd::runtime().spawn(async move {
                    let response = systemd::daemon_reload(dbus_level).await;
                    if let Err(e) = sender.send(response) {
                        error!("Channel closed unexpectedly: {e:?}");
                    }
                });

                let Ok(response) = receiver
                    .await
                    .inspect_err(|err| error!("Tokio channel dropped {err:?}"))
                else {
                    return;
                };

                match response {
                    Ok(_) => {
                        info!("All units reloaded! User session {:?}", dbus_level);
                        let instance_level = dbus_level.message();

                        let instance_level = format!("<b>{}</b>", instance_level);
                        let msg = format2!(
                            "Systemd manager configuration reloaded at {} level!",
                            instance_level
                        );
                        window.add_toast_message(&msg, true, None);

                        if page.imp().enable_switch.is_active() {
                            page.imp().enable_unit(&window, dbus_level);
                        }

                        if page.imp().start_switch.is_active() {
                            page.imp().start_unit(&window, dbus_level);
                        }
                    }
                    Err(e) => {
                        error!("Daemon Reload level {dbus_level:?} failed {e:?}");
                        //Faild to reload manager a System or User level
                        let msg = gettext("Daemon Reload failed at {} level");
                        let msg = format2!(msg, dbus_level.message());
                        let msg = format!("<red>{msg}</red>");
                        window.add_toast_message(&msg, true, None);
                    }
                }
                // simple_action.set_enabled(true);
            });
        }

        fn enable_unit(&self, window: &UnitCreatorWindow, dbus_level: UnitDBusLevel) {
            info!("enable");
        }

        fn start_unit(&self, window: &UnitCreatorWindow, dbus_level: UnitDBusLevel) {
            info!("enable");
        }
    }
    #[gtk::template_callbacks]
    impl LaunchCreatorPageImp {
        pub(crate) fn update_page(&self) {
            let window = upgrade_opt!(self.window.get());

            match window.creation_type() {
                UnitCreateType::Service => {
                    self.service_file_action.set_visible(true);
                    self.timer_file_action.set_visible(false);

                    if let Some(file_path) = window.imp().service_file_path() {
                        self.service_file_action.set_subtitle(&file_path);
                    }
                }
                UnitCreateType::Timer => {
                    self.service_file_action.set_visible(false);
                    self.timer_file_action.set_visible(true);

                    if let Some(file_path) = window.imp().timer_file_path() {
                        self.timer_file_action.set_subtitle(&file_path);
                    }
                }
                UnitCreateType::TimerService => {
                    self.service_file_action.set_visible(true);
                    self.timer_file_action.set_visible(true);

                    if let Some(file_path) = window.imp().service_file_path() {
                        self.service_file_action.set_subtitle(&file_path);
                    }

                    if let Some(file_path) = window.imp().timer_file_path() {
                        self.timer_file_action.set_subtitle(&file_path);
                    }
                }
            }
        }

        #[template_callback]
        fn show_service_file(&self, _button: &gtk::Button) {
            self.show_file(UnitCreatorWindowImp::service_file_path);
        }

        #[template_callback]
        fn show_timer_file(&self, _button: &gtk::Button) {
            self.show_file(UnitCreatorWindowImp::timer_file_path);
        }

        fn show_file(&self, call: fn(&UnitCreatorWindowImp) -> Option<String>) {
            let window = upgrade_opt!(self.window.get());
            if let Some(file_path) = call(window.imp()) {
                let uri = gio::File::for_uri(&format!("file://{file_path}"));
                let launcher = gtk::FileLauncher::new(Some(&uri));
                launcher.launch(Some(&window), None::<&gio::Cancellable>, move |result| {
                    if let Err(error) = result {
                        warn!("File launch Support Error {error:?}")
                    }
                });
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LaunchCreatorPageImp {
        const NAME: &'static str = "LaunchCreatorPage";
        type Type = LaunchCreatorPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LaunchCreatorPageImp {
        fn constructed(&self) {
            self.parent_constructed();

            let daemon_reload_active = self.daemon_reload_switch.is_active();
            self.enable_switch.set_sensitive(daemon_reload_active);
            self.start_switch.set_sensitive(daemon_reload_active);
        }
    }

    impl WidgetImpl for LaunchCreatorPageImp {}

    impl NavigationPageImpl for LaunchCreatorPageImp {}
}
