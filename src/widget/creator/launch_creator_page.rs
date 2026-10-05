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
        format2, systemd_gui, upgrade, upgrade_opt,
        widget::creator::{UnitCreateType, imp::UnitCreatorWindowImp},
    };
    use adw::{prelude::*, subclass::prelude::*};
    use base::{
        enums::UnitDBusLevel,
        file::{self},
    };
    use enumflags2::BitFlag;
    use gettextrs::{gettext, pgettext};
    use glib::clone::Downgrade;
    use gtk::glib;
    use std::cell::{Cell, OnceCell, RefCell};
    use systemd::enums::{DisEnableFlags, StartStopMode};
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
        artefacts_group: TemplateChild<adw::PreferencesGroup>,
        pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,
        rows: RefCell<Vec<InfoRow>>,
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

            for row in self.rows.borrow().iter() {
                row.file_button.set_sensitive(true);
                row.unit_button.set_sensitive(true);
            }

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

        fn enable_unit(&self, window: &UnitCreatorWindow, level: UnitDBusLevel) {
            match window.creation_type() {
                UnitCreateType::TimerService => {
                    self.enable_unit_call(window, level, UnitCreateType::Service);
                    self.enable_unit_call(window, level, UnitCreateType::Timer);
                }
                ct => {
                    self.enable_unit_call(window, level, ct);
                }
            }
        }

        fn enable_unit_call(
            &self,
            window: &UnitCreatorWindow,
            level: UnitDBusLevel,
            ct: UnitCreateType,
        ) {
            let unit_name = UnitCreatorWindowImp::unit_name(window.imp(), Some(ct));

            info!("enabling unit {:?}", unit_name);

            let flags = DisEnableFlags::empty();
            glib::spawn_future_local(async move {
                if let Err(err) = systemd::enable_unit_file(level, &unit_name, flags) {
                    warn!("Can't enable unit {:?}, Error {:?}", unit_name, err);
                }
            });
        }

        fn start_unit(&self, window: &UnitCreatorWindow, level: UnitDBusLevel) {
            match window.creation_type() {
                UnitCreateType::TimerService => {
                    self.start_unit_call(window, level, UnitCreateType::Service);
                    self.start_unit_call(window, level, UnitCreateType::Timer);
                }
                create_type => self.start_unit_call(window, level, create_type),
            }
        }

        fn start_unit_call(
            &self,
            window: &UnitCreatorWindow,
            level: UnitDBusLevel,
            create_type: UnitCreateType,
        ) {
            let unit_name = window.imp().unit_name(Some(create_type));
            info!("Starting unit {:?}", unit_name);

            glib::spawn_future_local(async move {
                if let Err(err) = systemd::start_unit(level, &unit_name, StartStopMode::Fail) {
                    warn!("Can't start unit {:?}, Error {:?}", unit_name, err);
                }
            });
        }
    }

    #[gtk::template_callbacks]
    impl LaunchCreatorPageImp {
        pub(crate) fn update_page(&self) {
            // let window = upgrade_opt!(self.window.get());
            let window = self.window.get().unwrap();
            let window = upgrade!(window);
            let creation_type = window.creation_type();

            // let pizza = Ve
            let rows = match creation_type {
                UnitCreateType::Service => {
                    vec![build_action_row(
                        //Action Row New Unit title
                        &pgettext("create_unit", "Service File"),
                        &window,
                        None,
                        &self.artefacts_group,
                    )]
                }
                UnitCreateType::Timer => {
                    vec![build_action_row(
                        //Action Row New Unit title
                        &pgettext("create_unit", "Service File"),
                        &window,
                        None,
                        &self.artefacts_group,
                    )]
                }
                UnitCreateType::TimerService => {
                    vec![
                        build_action_row(
                            //Action Row New Unit title
                            &pgettext("create_unit", "Service File"),
                            &window,
                            None,
                            &self.artefacts_group,
                        ),
                        build_action_row(
                            //Action Row New Unit title
                            &pgettext("create_unit", "Service File"),
                            &window,
                            None,
                            &self.artefacts_group,
                        ),
                    ]
                }
                UnitCreateType::Mount => {
                    vec![build_action_row(
                        //Action Row New Unit title
                        &pgettext("create_unit", "Mount File"),
                        &window,
                        None,
                        &self.artefacts_group,
                    )]
                }
            };
            self.rows.replace(rows);
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

            let settings = systemd_gui::new_settings();
            settings
                .bind(
                    "create-daemon-reload-after-creation",
                    &self.daemon_reload_switch.get(),
                    "active",
                )
                .build();
            settings
                .bind(
                    "create-enable-after-creation",
                    &self.enable_switch.get(),
                    "active",
                )
                .build();
            settings
                .bind(
                    "create-start-after-creation",
                    &self.start_switch.get(),
                    "active",
                )
                .build();
        }
    }

    impl WidgetImpl for LaunchCreatorPageImp {}

    impl NavigationPageImpl for LaunchCreatorPageImp {}

    struct InfoRow {
        file_button: gtk::Button,
        unit_button: gtk::Button,
    }

    fn build_action_row(
        row_title: &str,
        window: &UnitCreatorWindow,
        creator_type: Option<UnitCreateType>,
        artefacts_group: &adw::PreferencesGroup,
    ) -> InfoRow {
        let action_row = adw::ActionRow::builder()
            .subtitle_selectable(true)
            .title(row_title)
            .build();

        if let Some(file_path) = window.imp().file_path(creator_type)
            && let Some(file_path) = file_path.to_str()
        {
            action_row.set_subtitle(file_path);
        }

        let show_file_button = gtk::Button::builder()
            .icon_name("document-text-symbolic")
            .sensitive(false)
            //Tooltip Created new unit file
            .tooltip_text(pgettext("create_unit", "Show Unit File"))
            .valign(gtk::Align::Center)
            .build();

        let window_wr = glib::object::ObjectExt::downgrade(window);
        show_file_button.connect_clicked(move |_| {
            let window = upgrade!(window_wr);
            show_file(window, creator_type)
        });
        action_row.add_suffix(&show_file_button);

        let show_unit_button = gtk::Button::builder()
            .label(pgettext("create_unit", "Unit"))
            .sensitive(false)
            //Tooltip Created new unit
            .tooltip_text(pgettext("create_unit", "Show Unit in Browser"))
            .valign(gtk::Align::Center)
            .build();

        let window_wr = glib::object::ObjectExt::downgrade(window);
        show_unit_button.connect_clicked(move |_| {
            let window = upgrade!(window_wr);
            show_unit(window, creator_type)
        });
        action_row.add_suffix(&show_unit_button);

        artefacts_group.add(&action_row);
        InfoRow {
            file_button: show_file_button,
            unit_button: show_unit_button,
        }
    }

    fn show_file(window: UnitCreatorWindow, create_type: Option<UnitCreateType>) {
        if let Some(file_path) = window.imp().file_path(create_type)
            && let Some(file_path) = file_path.to_str()
        {
            let file_path = file::flatpak_host_file_path(file_path);
            let uri = gio::File::for_uri(&format!("file://{}", file_path.display()));
            let launcher = gtk::FileLauncher::new(Some(&uri));
            launcher.launch(Some(&window), None::<&gio::Cancellable>, move |result| {
                if let Err(error) = result {
                    warn!(
                        "File {:?} launch Support Error {error:?}",
                        file_path.display()
                    )
                }
            });
        }
    }

    fn show_unit(window: UnitCreatorWindow, unit_create_type: Option<UnitCreateType>) {
        let unit_name = window.unit_name(unit_create_type);

        let level = window.level();

        info!("Opening unit {:?} at level {:?}", unit_name, level);

        let unit = systemd::fetch_unit(level, &unit_name)
            .inspect_err(|e| warn!("Cli unit: {e:?}"))
            .ok();

        if let Some(app_window) = window.app_window() {
            app_window.set_unit(unit.as_ref());
        } else {
            warn!("app_window missing");
        }
    }
}
