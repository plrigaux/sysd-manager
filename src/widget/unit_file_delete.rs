use glib::subclass::types::ObjectSubclassIsExt;

use crate::widget::unit_control_panel::UnitControlPanel;

glib::wrapper! {
    pub struct UnitFileDeleteDialog(ObjectSubclass<imp::UnitFileDeleteDialogImp>)
        @extends  adw::Window, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;

}

impl UnitFileDeleteDialog {
    pub fn new(
        file_path: String,
        unit_name: &str,
        unit_control: UnitControlPanel,
    ) -> UnitFileDeleteDialog {
        let obj: UnitFileDeleteDialog = glib::Object::new();
        obj.imp().set_data(file_path, unit_name, unit_control);
        obj
    }
}

mod imp {
    use crate::{upgrade, widget::unit_control_panel::UnitControlPanel};
    use adw::{prelude::*, subclass::window::AdwWindowImpl};
    use base::enums::UnitDBusLevel;
    use gettextrs::pgettext;
    use glib::subclass::{
        object::ObjectImpl,
        types::{ObjectSubclass, ObjectSubclassExt},
    };
    use gtk::subclass::prelude::*;
    use std::cell::OnceCell;
    use systemd::{data::UnitInfo, errors::SystemdErrors};
    use tracing::{error, info};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_delete.ui")]
    pub struct UnitFileDeleteDialogImp {
        #[template_child]
        cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        delete_button: TemplateChild<gtk::Button>,
        #[template_child]
        file_path_link: TemplateChild<gtk::LinkButton>,
        #[template_child]
        are_you_sure_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        window_title: TemplateChild<adw::WindowTitle>,

        pub(super) file_path: OnceCell<String>,
        unit_control: OnceCell<UnitControlPanel>,
    }

    #[gtk::template_callbacks]
    impl UnitFileDeleteDialogImp {
        #[template_callback]
        fn response_cb(&self, response: &str) {
            info!("Unit delete response {}", response)
        }

        pub(super) fn set_data(
            &self,
            file_path: String,
            unit_name: &str,
            unit_control: UnitControlPanel,
        ) {
            self.window_title.set_subtitle(unit_name);
            self.file_path_link
                .set_uri(&format!("file://{}", file_path));
            self.file_path_link.set_label(&file_path);
            let _ = self.file_path.set(file_path);
            let _ = self.unit_control.set(unit_control);
        }

        fn delete_button_clicked(&self, button: &gtk::Button) {
            let lambda_out = {
                let this = self.obj().clone();
                move |method: &str,
                      unit: Option<&UnitInfo>,
                      result: Result<(), SystemdErrors>,
                      _control: &UnitControlPanel| {
                    if let Err(error) = result {
                        error!("Delete Unit File {:?}", error)
                    }
                }
            };

            let Some(file_path) = self.file_path.get() else {
                error!("No file path!");
                return;
            };

            let file_path = file_path.clone();
            let lambda = move |params: Option<(UnitDBusLevel, String)>| {
                if let Some((level, _)) = params {
                    systemd::runtime().block_on(systemd::delete_unit_file(level, &file_path))
                } else {
                    Err(SystemdErrors::NoUnit)
                }
            };

            self.unit_control
                .get()
                .expect("unit_control not None")
                .call_method(
                    /*Message answer*/ &pgettext("delete", "Delete Unit File"),
                    true,
                    button,
                    lambda,
                    lambda_out,
                );
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnitFileDeleteDialogImp {
        const NAME: &'static str = "UnitFileDeleteDialog";
        type Type = super::UnitFileDeleteDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnitFileDeleteDialogImp {
        fn constructed(&self) {
            self.parent_constructed();
            let this = self.obj().downgrade();
            self.cancel_button.connect_clicked(move |_| {
                let this = upgrade!(this);
                this.close();
            });

            let this = self.downgrade();
            self.delete_button.connect_clicked(move |button| {
                let this = upgrade!(this);
                this.delete_button_clicked(button);
            });

            let this = self.downgrade();
            self.are_you_sure_switch
                .connect_active_notify(move |switch| {
                    let this = upgrade!(this);
                    this.delete_button.set_sensitive(switch.is_active());
                });
        }
    }

    impl WidgetImpl for UnitFileDeleteDialogImp {}
    impl WindowImpl for UnitFileDeleteDialogImp {
        // Save window state right before the window will be closed
        fn close_request(&self) -> glib::Propagation {
            self.parent_close_request();

            glib::Propagation::Proceed
        }
    }
    impl AdwWindowImpl for UnitFileDeleteDialogImp {}
}
