use gtk::glib::{self};

glib::wrapper! {

    pub struct ServiceCreatorPage(ObjectSubclass<imp::ServiceCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl ServiceCreatorPage {
    pub fn new() -> Self {
        let obj: ServiceCreatorPage = glib::Object::new();
        obj
    }
}

impl Default for ServiceCreatorPage {
    fn default() -> Self {
        ServiceCreatorPage::new()
    }
}

mod imp {

    use super::*;
    use crate::widget::creator::UnitCreateType;
    use adw::subclass::prelude::*;
    use gtk::{glib, prelude::*};
    use std::cell::Cell;
    use tracing::warn;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/service_creator_page.ui")]
    #[properties(wrapper_type = super::ServiceCreatorPage)]
    pub struct ServiceCreatorPageImp {
        #[property(get, set, default)]
        creation_type: Cell<UnitCreateType>,

        #[template_child]
        executable_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        working_directory_entry: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServiceCreatorPageImp {
        const NAME: &'static str = "ServiceCreatorPage";
        type Type = ServiceCreatorPage;
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
    impl ObjectImpl for ServiceCreatorPageImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    #[gtk::template_callbacks]
    impl ServiceCreatorPageImp {
        #[template_callback]
        fn working_directory_search_dialog_clicked(&self, _button: gtk::Button) {
            let file_dialog = gtk::FileDialog::builder()
                .title("Select a working directory")
                .accept_label("Select")
                .build();

            let create_service_page = self.obj().clone();

            file_dialog.select_folder(
                None::<&gtk::Window>,
                None::<&gio::Cancellable>,
                move |result| match result {
                    Ok(file) => {
                        if let Some(path) = file.path() {
                            let file_path_str = path.display().to_string();
                            create_service_page
                                .imp()
                                .working_directory_entry
                                .set_text(&file_path_str);
                        }
                    }
                    Err(e) => warn!("Unit File Selection Error {e:?}"),
                },
            );
        }
    }

    impl WidgetImpl for ServiceCreatorPageImp {}

    impl NavigationPageImpl for ServiceCreatorPageImp {}
}
