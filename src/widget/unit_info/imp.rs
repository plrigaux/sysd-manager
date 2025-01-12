use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gtk::{
    gio,
    glib::{self},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
    FileLauncher, TemplateChild,
};

use log::{info, warn};

use crate::{
    systemd::{data::UnitInfo, generate_file_uri},
    widget::{app_window::AppWindow, info_window::InfoWindow},
};

use super::{
    construct_info::fill_all_info,
    text_view_hyperlink::{self, LinkActivator},
    writer::UnitInfoWriter,
};

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_info_panel.ui")]
pub struct UnitInfoPanelImp {
    #[template_child]
    show_all_button: TemplateChild<gtk::Button>,

    #[template_child]
    refresh_button: TemplateChild<gtk::Button>,

    #[template_child]
    unit_info_textview: TemplateChild<gtk::TextView>,

    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    hovering_over_link: Rc<Cell<bool>>,
}

#[gtk::template_callbacks]
impl UnitInfoPanelImp {
    #[template_callback]
    fn refresh_info_clicked(&self, button: &gtk::Button) {
        info!("button {:?}", button);

        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return;
        };

        self.update_unit_info(unit)
    }

    #[template_callback]
    fn show_all_clicked(&self, _button: &gtk::Button) {
        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return;
        };

        let info_window = InfoWindow::new();

        info!("show_all_clicked {:?}", unit.primary());

        info_window.fill_data(unit);

        info_window.present();
    }

    pub(crate) fn display_unit_info(&self, unit: &UnitInfo) {
        let _old = self.unit.replace(Some(unit.clone()));

        self.update_unit_info(unit)
    }

    /// Updates the associated journal `TextView` with the contents of the unit's journal log.
    fn update_unit_info(&self, unit: &UnitInfo) {
        let unit_info_text_view: &gtk::TextView = self.unit_info_textview.as_ref();

        let buf = unit_info_text_view.buffer();

        buf.set_text(""); // clear text

        let start_iter = buf.start_iter();

        let is_dark = self.is_dark.get();

        let mut info_writer = UnitInfoWriter::new(buf, start_iter, is_dark);

        fill_all_info(unit, &mut info_writer);
    }

    pub(crate) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitInfoPanelImp {
    const NAME: &'static str = "UnitInfoPanel";
    type Type = super::UnitInfoPanel;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for UnitInfoPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        let activator = LinkActivator::new(activate_link, None);

        text_view_hyperlink::build_textview_link_platform(
            &self.unit_info_textview,
            self.hovering_over_link.clone(),
            activator,
        );
    }
}
impl WidgetImpl for UnitInfoPanelImp {}
impl BoxImpl for UnitInfoPanelImp {}

fn activate_link(file_link: &str, _app_window: &Option<AppWindow>) {
    let uri = generate_file_uri(file_link);
    let file = gio::File::for_uri(&uri);
    let launcher = FileLauncher::new(Some(&file));
    launcher.launch(
        None::<&gtk::Window>,
        None::<&gio::Cancellable>,
        move |result| {
            if let Err(error) = result {
                warn!("Finished launch {} Error {:?}", uri, error)
            }
        },
    );
}
