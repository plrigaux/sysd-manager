use glib::subclass::types::ObjectSubclassIsExt;

glib::wrapper! {
    pub struct UnitFileDeleteDialog(ObjectSubclass<imp::UnitFileDeleteDialogImp>)
        @extends  adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl UnitFileDeleteDialog {
    pub fn new(file_path: String) -> UnitFileDeleteDialog {
        let obj: UnitFileDeleteDialog = glib::Object::new();
        obj.imp().set_file_path(file_path);
        obj
    }
}

mod imp {
    use std::cell::OnceCell;

    use crate::upgrade;
    use adw::{prelude::*, subclass::dialog::AdwDialogImpl};
    use glib::subclass::{
        object::ObjectImpl,
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
    };
    use gtk::subclass::widget::*;
    use tracing::info;

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

        pub(super) file_path: OnceCell<String>,
    }

    #[gtk::template_callbacks]
    impl UnitFileDeleteDialogImp {
        #[template_callback]
        fn response_cb(&self, response: &str) {
            info!("Unit delete response {}", response)
        }

        pub(super) fn set_file_path(&self, file_path: String) {
            self.file_path_link
                .set_uri(&format!("file://{}", file_path));
            self.file_path_link.set_label(&file_path);
            let _ = self.file_path.set(file_path);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnitFileDeleteDialogImp {
        const NAME: &'static str = "UnitFileDeleteDialog";
        type Type = super::UnitFileDeleteDialog;
        type ParentType = adw::Dialog;

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
            let this = self.obj().downgrade();
            self.cancel_button.connect_clicked(move |_| {
                let this = upgrade!(this);
                this.force_close();
            });

            let this = self.obj().downgrade();
            self.delete_button.connect_clicked(move |_| {
                let this = upgrade!(this);
                if let Some(file_path) = this.imp().file_path.get() {
                    info!("Deleting {:?}", file_path);
                }
                this.force_close();
            });

            let this = self.downgrade();
            self.are_you_sure_switch
                .connect_active_notify(move |switch| {
                    let this = upgrade!(this);
                    this.delete_button.set_sensitive(switch.is_active());
                });
        }
    }

    impl AdwDialogImpl for UnitFileDeleteDialogImp {}
    impl WidgetImpl for UnitFileDeleteDialogImp {}
}
