use super::CreatorPageTimer;
use crate::{
    upgrade, upgrade_opt,
    widget::{
        creator::{
            CreateUnitErr, UnitCreateType, UnitCreatorWindow,
            common::{self},
            creator_page_timer::{
                MonotonicTimer,
                validator::{self},
            },
            mydropdown::MyDropDown,
            suggestion::SuggestionRow,
            unit_file::{ON_CALENDAR, TIMER, UnitFileData},
            unit_file_creator_page::UnitFileCreatorPage,
        },
        find_child_by_name,
    },
};
use adw::{prelude::*, subclass::prelude::*};
use gettextrs::pgettext;
use glib::{VariantTy, WeakRef};
use gtk::{
    glib::{self},
    prelude::{ButtonExt, EditableExt, ObjectExt, WidgetExt},
};
use std::{
    borrow::Cow,
    cell::{Cell, OnceCell, RefCell},
    collections::HashSet,
};
use strum::{EnumIter, IntoEnumIterator};
use tracing::error;
const ACTION_CREATOR_MONOTONIC_ADD: &str = "creator.monotonic-add";
const ACTION_CREATOR_REALTIME_ADD: &str = "creator.realtime-add";

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/creator_page_timer.ui")]
#[properties(wrapper_type = super::CreatorPageTimer)]
pub struct CreatorPageTimerImp {
    #[property(get, default)]
    creation_type: Cell<UnitCreateType>,

    #[template_child]
    trigger_unit: TemplateChild<MyDropDown>,

    #[template_child]
    description: TemplateChild<adw::EntryRow>,

    #[template_child]
    monotonic_timer_adder: TemplateChild<adw::SplitButton>,

    #[template_child]
    persistent: TemplateChild<adw::SwitchRow>,

    #[template_child]
    realtime_timer_adder: TemplateChild<adw::SplitButton>,

    #[template_child]
    timers_group: TemplateChild<adw::PreferencesGroup>,

    #[template_child]
    wanted_by_entry: TemplateChild<SuggestionRow>,

    #[template_child]
    install_group: TemplateChild<adw::PreferencesGroup>,

    pub(super) file_data: RefCell<UnitFileData>,

    pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

    monotonic_type: Cell<MonotonicTimer>,
    realtime_type: Cell<RealTimeTimer>,

    pub monotonic_timers: RefCell<Vec<(String, adw::EntryRow)>>,
    pub realtime_timers: RefCell<Vec<adw::EntryRow>>,
}

#[glib::object_subclass]
impl ObjectSubclass for CreatorPageTimerImp {
    const NAME: &'static str = "CreatorPageTimer";
    type Type = CreatorPageTimer;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        //To force the read
        // widget::creator::dropdown::SysDDropDown::default();
        klass.bind_template();
        //klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for CreatorPageTimerImp {
    fn constructed(&self) {
        self.parent_constructed();
        self.trigger_unit.set_enable_search(true);

        let menu = gio::Menu::new();

        for timer in MonotonicTimer::iter() {
            add_menu_item_param(
                &menu,
                &timer.label(),
                ACTION_CREATOR_MONOTONIC_ADD,
                timer.param(),
            );
        }

        self.monotonic_timer_adder.set_menu_model(Some(&menu));

        let menu = gio::Menu::new();
        for timer in RealTimeTimer::iter() {
            add_menu_item_param(
                &menu,
                &timer.label(),
                ACTION_CREATOR_REALTIME_ADD,
                timer.param(),
            );
        }

        let timer_panel = self.obj().clone();
        self.monotonic_timer_adder.connect_clicked(move |_| {
            timer_panel.imp().add_monotonic();
        });
        self.realtime_timer_adder.set_menu_model(Some(&menu));
        let timer_panel = self.obj().clone();
        self.realtime_timer_adder.connect_clicked(move |_| {
            timer_panel.imp().add_realtime();
        });
        self.select_add_monotonic(MonotonicTimer::default());
        self.select_add_realtime(RealTimeTimer::default());

        self.description
            .connect_has_focus_notify(|entry| entry.select_region(0, -1));
        self.description
            .connect_focus_on_click_notify(|entry| entry.select_region(0, -1));

        let event_controller = gtk::EventControllerFocus::new();
        let this = self.downgrade();
        event_controller.connect_leave(move |_event| {
            let this = upgrade!(this);

            this.validate_unit_wanted_by();
        });
        self.wanted_by_entry.add_controller(event_controller);
        let event_controller = gtk::EventControllerFocus::new();
        let this = self.downgrade();
        event_controller.connect_leave(move |_event| {
            let this = upgrade!(this);

            this.validate_unit();
        });
        self.trigger_unit.add_controller(event_controller);

        self.obj().connect_showing(|page| {
            page.imp().validate();
        });
    }
}

impl CreatorPageTimerImp {
    pub(super) fn validate(&self) {
        self.validate_unit_wanted_by();
        self.validate_unit();
    }

    fn validate_unit_wanted_by(&self) {
        common::validate_unit_common(
            &self.wanted_by_entry.get(),
            "WantedBy",
            Some(&self.wanted_by_entry.text()),
            self.window(),
        );
    }

    fn validate_unit(&self) {
        let preference = &self.trigger_unit.get();
        let prefix = "Unit";
        let unit_name = preference.subtitle().unwrap_or_default();
        let unit_name = unit_name.trim();

        let name_err = if unit_name.is_empty() {
            match self.creation_type.get() {
                UnitCreateType::Timer => {
                    if let Some(window) = self.window.get()
                        && let Some(win) = window.upgrade()
                    {
                        let unit_name = win.unit_name(UnitCreateType::Service);
                        match common::is_fill_exist(self.window(), &unit_name) {
                            Ok(true) => CreateUnitErr::NoErr,
                            Ok(false) => CreateUnitErr::NotUnit,
                            Err(e) => e,
                        }
                    } else {
                        error!("No Window");
                        CreateUnitErr::NoErr
                    }
                }
                UnitCreateType::TimerService => CreateUnitErr::NoErr,
                ct => {
                    error!("Invalid Creation Type {ct:?}");
                    CreateUnitErr::NoErr
                }
            }
        } else {
            match common::is_fill_exist(self.window(), unit_name) {
                Ok(true) => CreateUnitErr::NoErr,
                Ok(false) => CreateUnitErr::NotUnit,
                Err(e) => e,
            }
        };
        CreateUnitErr::apply_validation_result(preference, &name_err, prefix);
    }

    fn window(&self) -> &WeakRef<UnitCreatorWindow> {
        self.window.get().unwrap()
    }

    pub(super) fn update_from_unit_info(&self) {
        let window = upgrade_opt!(self.window.get());

        let model = window.imp().get_trigger_units_model();

        let single_selection_model = gtk::SingleSelection::builder()
            .can_unselect(true)
            .autoselect(false)
            .model(&model)
            .build();

        let filter = gtk::CustomFilter::new(|object| {
            let Some(string_object) = object.downcast_ref::<gtk::StringObject>() else {
                return false;
            };

            !string_object.string().ends_with(".timer")
        });

        let filtered_model =
            gtk::FilterListModel::new(Some(single_selection_model), Some(filter.clone()));
        // self.trigger_unit.set_selected(gtk::INVALID_LIST_POSITION);
        self.trigger_unit.set_model(Some(&filtered_model));

        let single_selection_model = gtk::SingleSelection::builder()
            .can_unselect(true)
            .autoselect(false)
            .model(&model)
            .build();
        self.wanted_by_entry
            .set_model(Some(&single_selection_model));
    }

    pub(super) fn create_actions(&self) {
        let window = upgrade_opt!(self.window.get());

        let monotonic_add: gio::ActionEntry<_> = {
            let timer_page = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_CREATOR_MONOTONIC_ADD[8..])
                .activate(move |_, _, v| {
                    let timer: MonotonicTimer = v.into();
                    timer_page.imp().select_add_monotonic(timer);
                    timer_page.imp().add_monotonic();
                })
                .parameter_type(Some(VariantTy::STRING))
                .build()
        };

        let realtime_add: gio::ActionEntry<_> = {
            let timer_page = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_CREATOR_REALTIME_ADD[8..])
                .activate(move |_, _, v| {
                    let calendar_type: RealTimeTimer = v.into();
                    timer_page.imp().select_add_realtime(calendar_type);
                    timer_page.imp().add_realtime();
                })
                .parameter_type(Some(VariantTy::STRING))
                .build()
        };

        let action_group = window.action_group();

        action_group.add_action_entries([monotonic_add, realtime_add]);
    }

    fn select_add_realtime(&self, calendar_type: RealTimeTimer) {
        self.realtime_timer_adder
            .set_label(&format!("Add {}", calendar_type.label()));
        self.realtime_type.set(calendar_type);
    }

    fn add_realtime(&self) {
        let calendar_type = self.realtime_type.get();
        self.add_realtime2(Some(&calendar_type.text()));
    }

    fn add_realtime2(&self, calendar_type: Option<&str>) {
        let entry_row = adw::EntryRow::builder()
            .title(ON_CALENDAR)
            .text(calendar_type.unwrap_or_default())
            .title_selectable(true)
            .show_apply_button(true)
            .build();

        entry_row.connect_has_focus_notify(|entry| entry.select_region(0, -1));
        entry_row.connect_focus_on_click_notify(|entry| entry.select_region(0, -1));
        entry_row.connect_apply(super::validator::validate_calendar_entry);

        let event_controller = gtk::EventControllerFocus::new();
        let entry_row_weak = entry_row.downgrade();
        event_controller.connect_leave(move |_| {
            let entry_row = upgrade!(entry_row_weak);

            if let Some(button) = find_child_by_name::<gtk::Button>(&entry_row, "apply_button") {
                button.emit_clicked();
            }
        });

        entry_row.add_controller(event_controller);

        let button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::BaselineCenter)
            .css_classes(["flat"])
            .build();

        entry_row.add_suffix(&button);
        let timers_panel = self.obj().downgrade();
        self.monotonic_timers
            .borrow_mut()
            .push((ON_CALENDAR.to_string(), entry_row.clone()));
        self.timers_group.add(&entry_row);
        button.connect_clicked(move |_| {
            let timers_panel = upgrade!(timers_panel);
            timers_panel.imp().remove_realtime(&entry_row);
        });
    }

    fn add_monotonic(&self) {
        let timer = self.monotonic_type.get();
        self.add_monotonic2(timer, None);
    }

    fn add_monotonic2(&self, timer: MonotonicTimer, value: Option<&str>) {
        let entry_row = adw::EntryRow::builder()
            .title(timer.label())
            .text(value.unwrap_or_default())
            .can_focus(true)
            .focusable(true)
            .title_selectable(true)
            .show_apply_button(true)
            .build();

        entry_row.connect_has_focus_notify(|entry| {
            dbg!(entry.has_focus());
            entry.select_region(0, -1)
        });
        entry_row.connect_focus_on_click_notify(|entry| entry.select_region(0, -1));
        entry_row.connect_move_focus(|e, a| println!("{:?} {}", a, e.text()));
        entry_row.connect_focusable_notify(|e| println!("foc {:?} ", e.text()));
        entry_row.connect_entry_activated(|f| println!("activated {}", f.has_focus()));
        entry_row
            .connect_apply(move |entry| validator::validate_monotonic_entry(timer.label(), entry));

        // let event_controller = widget::clear_on_escape_entry_row();
        // entry_row.add_controller(event_controller);

        let event_controller = gtk::EventControllerFocus::new();

        let entry_row_weak = entry_row.downgrade();
        event_controller.connect_leave(move |_| {
            let entry_row = upgrade!(entry_row_weak);

            if let Some(button) = find_child_by_name::<gtk::Button>(&entry_row, "apply_button") {
                button.emit_clicked();
            }
        });

        entry_row.add_controller(event_controller);

        let button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::BaselineCenter)
            .css_classes(["flat"])
            .build();

        entry_row.add_suffix(&button);
        let timers_group = self.obj().downgrade();
        self.timers_group.add(&entry_row);

        self.monotonic_timers
            .borrow_mut()
            .push((timer.param().to_string(), entry_row.clone()));

        button.connect_clicked(move |_| {
            let timers_panel = upgrade!(timers_group);
            timers_panel.imp().remove_monotonic(&entry_row);
        });
    }

    fn remove_monotonic(&self, entry_row: &adw::EntryRow) {
        self.timers_group.remove(entry_row);

        let mut vec = self.monotonic_timers.borrow_mut();
        vec.retain(|(_, e)| e != entry_row);
    }

    fn remove_realtime(&self, entry_row: &adw::EntryRow) {
        self.timers_group.remove(entry_row);

        let mut vec = self.realtime_timers.borrow_mut();
        vec.retain(|e| e != entry_row);
    }

    fn select_add_monotonic(&self, timer: MonotonicTimer) {
        self.monotonic_timer_adder
            .set_label(&format!("Add {}", timer.label()));
        self.monotonic_type.set(timer);
    }

    pub fn set_creation_type(&self, creation_type: UnitCreateType) {
        match creation_type {
            UnitCreateType::Service => {}
            UnitCreateType::Timer => {
                self.trigger_unit.set_visible(true);
            }
            UnitCreateType::TimerService => {
                self.trigger_unit.set_visible(false);
                self.trigger_unit.set_subtitle("");
                self.file_data.borrow_mut().remove_trigger_unit();
            }
            UnitCreateType::Mount => {}
        }

        self.creation_type.set(creation_type);
    }

    pub fn advanced_mode(&self, advanced: bool) {
        self.install_group.set_visible(advanced);
    }

    pub fn update_view(&self, page: &UnitFileCreatorPage) {
        self.fill_data();
        let data = self.file_data.borrow();
        page.update_view(&data);
    }

    fn fill_data(&self) {
        let mut file_data = self.file_data.borrow_mut();

        file_data.set_description(self.description.text());
        file_data.set_persistent(self.persistent.is_active());
        file_data.set_wanted_by(self.wanted_by_entry.text());

        let timers = self
            .monotonic_timers
            .borrow()
            .iter()
            .filter(|(_, e)| !e.text().trim_ascii().is_empty())
            .map(|(id, entry)| (id.clone(), entry.text().trim_ascii().to_string()))
            .collect::<Vec<_>>();

        let mut set = HashSet::from([ON_CALENDAR.to_string()]);
        for s in MonotonicTimer::iter() {
            set.insert(s.param().to_string());
        }

        let mut timer_map: indexmap::IndexMap<String, Vec<String>> = indexmap::IndexMap::new();

        for (timer, value) in timers.into_iter() {
            set.remove(&timer);
            match timer_map.entry(timer) {
                indexmap::map::Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().push(value);
                }
                indexmap::map::Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert_entry(vec![value]);
                }
            };
        }

        file_data.add_timers(timer_map);

        for s in set {
            file_data.remove(TIMER, &s);
        }

        file_data.set_trigger_unit(self.trigger_unit.subtitle());

        file_data.sort();
    }

    pub(super) fn file_content(&self) -> String {
        self.fill_data();
        self.file_data.borrow().to_file()
    }

    pub(super) fn update_from_file_content(&self, content: &str) {
        let Some(data) = UnitFileData::from_content(content) else {
            return;
        };

        let window = upgrade_opt!(self.window.get());

        self.description.set_text(data.description());
        self.persistent.set_active(data.persistent());
        self.wanted_by_entry.set_text(data.wanted_by());

        if matches!(window.creation_type(), UnitCreateType::Timer) {
            self.trigger_unit.set_subtitle(data.trigger_unit());
        } else {
            self.trigger_unit.set_subtitle("");
        }

        for (_, entry_row) in self.monotonic_timers.borrow_mut().drain(..) {
            self.timers_group.remove(&entry_row);
        }

        for (timer, values) in data.timers() {
            if let Some(m_timer) = MonotonicTimer::get(&timer.attribute) {
                for value in values {
                    self.add_monotonic2(m_timer, Some(value.as_str()));
                }
            } else {
                for value in values {
                    self.add_realtime2(Some(value.as_str()));
                }
            }
        }

        self.wanted_by_entry.set_text(data.wanted_by());

        self.file_data.replace(data);
    }
}

fn add_menu_item_param(menu: &gio::Menu, label: &str, action: &str, param: &str) {
    add_menu_item(menu, label, action, Some(param));
}

fn add_menu_item(menu: &gio::Menu, label: &str, action: &str, param: Option<&str>) {
    let action = if let Some(param) = param {
        Cow::Owned(format!("{action}::{param}"))
    } else {
        Cow::Borrowed(action)
    };

    let item = gio::MenuItem::new(Some(label), Some(&action));
    menu.append_item(&item);
}

impl WidgetImpl for CreatorPageTimerImp {}

impl NavigationPageImpl for CreatorPageTimerImp {}

#[derive(Debug, Copy, Clone, Default, EnumIter)]
enum RealTimeTimer {
    #[default]
    Custom,
    Minutely,
    Hourly,
    Daily,
    Monthly,
    Weekly,
    Yearly,
    Quarterly,
    Semiannually,
}

impl RealTimeTimer {
    fn param(&self) -> &str {
        match self {
            RealTimeTimer::Custom => "Custom",
            RealTimeTimer::Minutely => "Minutely",
            RealTimeTimer::Hourly => "Hourly",
            RealTimeTimer::Daily => "Daily",
            RealTimeTimer::Monthly => "Monthly",
            RealTimeTimer::Weekly => "Weekly",
            RealTimeTimer::Yearly => "Yearly",
            RealTimeTimer::Quarterly => "Quarterly",
            RealTimeTimer::Semiannually => "Semiannually",
        }
    }

    fn text(&self) -> String {
        match self {
            RealTimeTimer::Custom => "*-*-* *:*:*".to_owned(),
            _ => self.param().to_lowercase(),
        }
    }
    fn label(&self) -> String {
        match self {
            RealTimeTimer::Custom => pgettext("timer", "Custom"),
            RealTimeTimer::Minutely => pgettext("timer", "Minutely"),
            RealTimeTimer::Hourly => pgettext("timer", "Hourly"),
            RealTimeTimer::Daily => pgettext("timer", "Daily"),
            RealTimeTimer::Monthly => pgettext("timer", "Monthly"),
            RealTimeTimer::Weekly => pgettext("timer", "Weekly"),
            RealTimeTimer::Yearly => pgettext("timer", "Yearly"),
            RealTimeTimer::Quarterly => pgettext("timer", "Quarterly"),
            RealTimeTimer::Semiannually => pgettext("timer", "Semiannually"),
        }
    }
}

impl From<Option<&glib::Variant>> for RealTimeTimer {
    fn from(value: Option<&glib::Variant>) -> Self {
        match value.and_then(|v| v.get::<String>()).as_deref() {
            Some("Custom") => Self::Custom,
            Some("Minutely") => Self::Minutely,
            Some("Hourly") => Self::Hourly,
            Some("Daily") => Self::Daily,
            Some("Monthly") => Self::Monthly,
            Some("Weekly") => Self::Weekly,
            Some("Yearly") => Self::Yearly,
            Some("Quarterly") => Self::Quarterly,
            Some("Semiannually") => Self::Semiannually,
            Some(_) | None => Self::default(),
        }
    }
}
