mod imp;
mod validator;
use crate::widget::creator::{
    PageType, UnitCreateType, UnitCreatorWindow, unit_file_creator_page::UnitFileCreatorPage,
};
use adw::prelude::NavigationPageExt;
use gettextrs::pgettext;
use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};
use strum::{EnumIter, IntoEnumIterator};

glib::wrapper! {
    pub struct TimerCreatorPage(ObjectSubclass<imp::TimerCreatorPageImp>)
    @extends adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget ;
}

impl TimerCreatorPage {
    pub fn new(window: WeakRef<UnitCreatorWindow>, page: PageType) -> Self {
        let obj: TimerCreatorPage = glib::Object::new();
        obj.set_tag(Some(page.id()));
        let _ = obj.imp().window.set(window);
        obj.imp().update_from_unit_info();
        obj.imp().create_actions();
        obj
    }

    pub fn update_from_unit_info(&self) {
        self.imp().update_from_unit_info();
    }

    pub fn update_view(&self, page: &UnitFileCreatorPage) {
        self.imp().update_view(page);
    }

    pub fn update_from_file_content(&self, content: &str) {
        self.imp().update_from_file_content(content);
    }

    pub fn file_content(&self) -> String {
        self.imp().file_content()
    }

    pub fn set_view(&self, creation_type: UnitCreateType) {
        self.imp().set_view(creation_type)
    }
}

#[derive(Debug, Copy, Clone, Default, EnumIter)]
pub enum MonotonicTimer {
    #[default]
    Active,
    Boot,
    Startup,
    UnitActive,
    UnitInactive,
}

impl MonotonicTimer {
    fn param(&self) -> &str {
        match self {
            MonotonicTimer::Active => "OnActiveSec",
            MonotonicTimer::Boot => "OnBootSec",
            MonotonicTimer::Startup => "OnStartupSec",
            MonotonicTimer::UnitActive => "OnUnitActiveSec",
            MonotonicTimer::UnitInactive => "OnUnitInactiveSec",
        }
    }

    fn label(&self) -> String {
        match self {
            MonotonicTimer::Active => pgettext("timer", "OnActiveSec"),
            MonotonicTimer::Boot => pgettext("timer", "OnBootSec"),
            MonotonicTimer::Startup => pgettext("timer", "OnStartupSec"),
            MonotonicTimer::UnitActive => pgettext("timer", "OnUnitActiveSec"),
            MonotonicTimer::UnitInactive => pgettext("timer", "OnUnitInactiveSec"),
        }
    }

    pub fn get(value: &str) -> Option<MonotonicTimer> {
        MonotonicTimer::iter().find(|t| t.param() == value)
    }
}

impl From<Option<&glib::Variant>> for MonotonicTimer {
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
