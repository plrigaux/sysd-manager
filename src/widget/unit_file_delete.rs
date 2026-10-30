glib::wrapper! {
    pub struct UnitFileDeleteDialog(ObjectSubclass<imp::UnitFileDeleteDialogImp>)
        @extends adw::AlertDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl UnitFileDeleteDialog {
    fn new() -> UnitFileDeleteDialog {
        let obj: UnitFileDeleteDialog = glib::Object::new();
        obj
    }
}

impl Default for UnitFileDeleteDialog {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use adw::subclass::{alert_dialog::AdwAlertDialogImpl, dialog::AdwDialogImpl};
    use glib::subclass::{object::ObjectImpl, types::ObjectSubclass};
    use gtk::subclass::widget::*;
    use tracing::info;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_delete.ui")]
    pub struct UnitFileDeleteDialogImp {}

    #[gtk::template_callbacks]
    impl UnitFileDeleteDialogImp {
        #[template_callback]
        fn response_cb(&self, response: &str) {
            info!("Unit delete response {}", response)
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnitFileDeleteDialogImp {
        const NAME: &'static str = "UnitFileDeleteDialog";
        type Type = super::UnitFileDeleteDialog;
        type ParentType = adw::AlertDialog;

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
        fn constructed(&self) {}
    }

    impl AdwAlertDialogImpl for UnitFileDeleteDialogImp {}
    impl AdwDialogImpl for UnitFileDeleteDialogImp {}
    impl WidgetImpl for UnitFileDeleteDialogImp {}
}
