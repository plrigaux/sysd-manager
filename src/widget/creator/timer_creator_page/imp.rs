use super::TimerCreatorPage;
use crate::{
    upgrade, upgrade_opt,
    widget::{
        self,
        creator::{UnitCreateType, UnitCreatorWindow},
    },
};
use adw::{
    prelude::{ComboRowExt, EntryRowExt, PreferencesGroupExt},
    subclass::prelude::*,
};
use gettextrs::pgettext;
use gio::prelude::*;
use glib::{VariantTy, WeakRef, clone::Downgrade};
use gtk::{
    glib::{self},
    prelude::{ButtonExt, ObjectExt, WidgetExt},
};
use std::{
    borrow::Cow,
    cell::{Cell, OnceCell},
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
    monotonic_timer_adder: TemplateChild<adw::SplitButton>,

    #[template_child]
    realtime_timer_adder: TemplateChild<adw::SplitButton>,

    #[template_child]
    timers_group: TemplateChild<adw::PreferencesGroup>,

    pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

    monotonic_type: Cell<MontotonicTimer>,
    realtime_type: Cell<RealTimeTimer>,
}

#[glib::object_subclass]
impl ObjectSubclass for TimerCreatorPageImp {
    const NAME: &'static str = "TimerCreatorPage";
    type Type = TimerCreatorPage;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
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

        self.trigger_unit.connect_selected_item_notify(|a| {
            dbg!("Connect idx {}", a.selected());
        });

        let menu = gio::Menu::new();

        for timer in MontotonicTimer::iter() {
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

        self.realtime_timer_adder.set_menu_model(Some(&menu));
        let timer_panel = self.obj().clone();
        self.realtime_timer_adder.connect_clicked(move |_| {
            timer_panel.imp().add_realtime();
        });
        self.select_add_monotonic(MontotonicTimer::default());
        self.select_add_realtime(RealTimeTimer::default());
    }
}

impl TimerCreatorPageImp {
    pub(super) fn update_from_unit_info(&self) {
        let window = upgrade_opt!(self.window.get());

        let set = window.imp().get_trigger_units();

        let mut vec = set
            .iter()
            .filter(|s| !s.ends_with(".timer"))
            .map(|s| s.as_ref())
            .collect::<Vec<_>>();
        vec.push(""); //for unselect
        vec.sort();

        let model = gtk::StringList::new(&vec);
        // self.trigger_unit.set_selected(gtk::INVALID_LIST_POSITION);
        self.trigger_unit.set_model(Some(&model));
        self.trigger_unit.set_selected(gtk::INVALID_LIST_POSITION);
    }

    pub(super) fn create_actions(&self) {
        let window = upgrade_opt!(self.window.get());

        let monotonic_add: gio::ActionEntry<_> = {
            let timer_page = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_CREATOR_MONOTONIC_ADD[8..])
                .activate(move |_, _, v| {
                    let timer: MontotonicTimer = v.into();
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
        let entry_row = adw::EntryRow::builder()
            .title("On Calendar")
            .text(calendar_type.text())
            .build();

        let event_controller = widget::clear_on_escape2();
        entry_row.add_controller(event_controller);

        let button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::BaselineCenter)
            .css_classes(["flat"])
            .build();

        entry_row.add_suffix(&button);
        let timers_group = self.timers_group.downgrade();
        self.timers_group.add(&entry_row);
        button.connect_clicked(move |_| {
            let timers_group = upgrade!(timers_group);
            timers_group.remove(&entry_row);
        });
    }

    fn add_monotonic(&self) {
        let timer = self.monotonic_type.get();
        let entry_row = adw::EntryRow::builder()
            .title(timer.label())
            .text("")
            .build();

        let event_controller = widget::clear_on_escape2();
        entry_row.add_controller(event_controller);

        let button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::BaselineCenter)
            .css_classes(["flat"])
            .build();

        entry_row.add_suffix(&button);
        let timers_group = self.timers_group.downgrade();
        self.timers_group.add(&entry_row);
        button.connect_clicked(move |_| {
            let timers_group = upgrade!(timers_group);
            timers_group.remove(&entry_row);
        });
    }

    fn select_add_monotonic(&self, timer: MontotonicTimer) {
        self.monotonic_timer_adder
            .set_label(&format!("Add {}", timer.label()));
        self.monotonic_type.set(timer);
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
enum MontotonicTimer {
    #[default]
    Active,
    Boot,
    Startup,
    UnitActive,
    UnitInactive,
}

impl MontotonicTimer {
    fn param(&self) -> &str {
        match self {
            MontotonicTimer::Active => "OnActiveSec",
            MontotonicTimer::Boot => "OnBootSec",
            MontotonicTimer::Startup => "OnStartupSec",
            MontotonicTimer::UnitActive => "OnUnitActiveSec",
            MontotonicTimer::UnitInactive => "OnUnitInactiveSec",
        }
    }

    fn label(&self) -> String {
        match self {
            MontotonicTimer::Active => pgettext("timer", "OnActiveSec "),
            MontotonicTimer::Boot => pgettext("timer", "OnBootSec "),
            MontotonicTimer::Startup => pgettext("timer", "OnStartupSec "),
            MontotonicTimer::UnitActive => pgettext("timer", "OnUnitActiveSec "),
            MontotonicTimer::UnitInactive => pgettext("timer", "OnUnitInactiveSec "),
        }
    }
}

impl From<Option<&glib::Variant>> for MontotonicTimer {
    fn from(value: Option<&glib::Variant>) -> Self {
        match value.and_then(|v| v.get::<String>()).as_deref() {
            Some("OnActiveSec") => Self::Active,
            Some("OnBootSec") => Self::Boot,
            Some("OnStartupSec") => Self::Startup,
            Some("OnUnitActiveSec") => Self::UnitActive,
            Some("OnUnitInactiveSec") => Self::UnitInactive,
            Some(_) | None => Self::default(),
        }
    }
}
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
