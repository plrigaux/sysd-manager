use super::TimerCreatorPage;
use crate::{
    upgrade, upgrade_opt,
    widget::{
        self,
        creator::{
            UnitCreateType, UnitCreatorWindow,
            dropdown::SysDDropDown,
            timer_creator_page::MonotonicTimer,
            unit_file::{ON_CALENDAR, TIMER, UnitFileData},
            unit_file_creator_page::UnitFileCreatorPage,
        },
    },
};
use adw::{
    prelude::{ActionRowExt, ComboRowExt, EntryRowExt, PreferencesGroupExt},
    subclass::prelude::*,
};
use gettextrs::pgettext;
use gio::prelude::*;
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
const ACTION_CREATOR_MONOTONIC_ADD: &str = "creator.monotonic-add";
const ACTION_CREATOR_REALTIME_ADD: &str = "creator.realtime-add";

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/timer_creator_page.ui")]
#[properties(wrapper_type = super::TimerCreatorPage)]
pub struct TimerCreatorPageImp {
    #[property(get, set, default)]
    creation_type: Cell<UnitCreateType>,

    #[template_child]
    trigger_unit: TemplateChild<adw::ComboRow>,

    #[template_child]
    trigger_unit2: TemplateChild<SysDDropDown>,

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

    pub(super) file_data: RefCell<UnitFileData>,

    pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

    monotonic_type: Cell<MonotonicTimer>,
    realtime_type: Cell<RealTimeTimer>,

    pub monotonic_timers: RefCell<Vec<(String, adw::EntryRow)>>,
    pub realtime_timers: RefCell<Vec<adw::EntryRow>>,
}

#[glib::object_subclass]
impl ObjectSubclass for TimerCreatorPageImp {
    const NAME: &'static str = "TimerCreatorPage";
    type Type = TimerCreatorPage;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        //To force the read
        widget::creator::dropdown::SysDDropDown::default();
        klass.bind_template();
        //klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for TimerCreatorPageImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.trigger_unit.connect_selected_item_notify(|_| {
            // dbg!("Connect idx {}", a.selected());
        });

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
    }
}

impl TimerCreatorPageImp {
    pub(super) fn update_from_unit_info(&self) {
        let window = upgrade_opt!(self.window.get());

        let set = window.imp().get_trigger_units();

        let mut vec = set
            .iter()
            //    .filter(|s| !s.ends_with(".timer"))
            .map(|s| s.as_ref())
            .collect::<Vec<_>>();
        vec.push(""); //for unselect
        vec.sort();

        let model = gtk::StringList::new(&vec);
        let model2 = gtk::SingleSelection::builder()
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

        let model3 = gtk::FilterListModel::new(Some(model2), Some(filter));
        // self.trigger_unit.set_selected(gtk::INVALID_LIST_POSITION);
        self.trigger_unit.set_model(Some(&model3));

        self.trigger_unit.set_selected(gtk::INVALID_LIST_POSITION);
        self.trigger_unit2.set_model(Some(&model3));
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
            .build();

        let event_controller = widget::clear_on_escape2();
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
            .build();

        let event_controller = widget::clear_on_escape2();
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

    pub fn set_view(&self, creation_type: UnitCreateType) {
        match creation_type {
            UnitCreateType::Service => {}
            UnitCreateType::Timer => {
                self.trigger_unit.set_visible(true);
                self.trigger_unit2.set_visible(true);
            }
            UnitCreateType::TimerService => {
                self.trigger_unit.set_visible(false);
                self.trigger_unit.set_subtitle("");
                self.file_data.borrow_mut().remove_trigger_unit();
                self.trigger_unit2.set_visible(false);
            }
        }
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
        file_data.set_trigger_unit(self.trigger_unit.subtitle());

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

        file_data.sort();
    }

    pub(super) fn file_content(&self) -> String {
        self.fill_data();
        self.file_data.borrow().to_file()
    }

    pub fn update_from_file_content(&self, content: &str) {
        let Some(data) = UnitFileData::from_content(content) else {
            return;
        };

        let window = upgrade_opt!(self.window.get());

        self.description.set_text(data.description());
        self.persistent.set_active(data.persistent());

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

impl WidgetImpl for TimerCreatorPageImp {}

impl NavigationPageImpl for TimerCreatorPageImp {}

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
