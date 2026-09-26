use crate::widget::creator::{
    UnitCreatorWindow, creator_page_mount::CreatorPageMount, suggestion::SuggestionRow,
    unit_file::UnitFileData, unit_file_creator_page::UnitFileCreatorPage,
};
use adw::{prelude::*, subclass::prelude::*};
use base::file::commander;
use glib::{
    WeakRef,
    object::{Cast, CastNone},
};
use regex::Regex;
use std::{
    cell::{OnceCell, RefCell},
    collections::BTreeSet,
};
use systemd::{errors::SystemdErrors, runtime};
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, BufReader},
};
use tracing::{debug, warn};

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/creator_page_mount.ui")]
#[properties(wrapper_type = super::CreatorPageMount)]
pub struct CreatorPageMountImp {
    #[template_child]
    mount_type_suggestion: TemplateChild<SuggestionRow>,

    pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

    pub(super) file_data: RefCell<UnitFileData>,

    file_system_names: RefCell<BTreeSet<String>>,
}

#[glib::object_subclass]
impl ObjectSubclass for CreatorPageMountImp {
    const NAME: &'static str = "CreatorPageMount";
    type Type = CreatorPageMount;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        SuggestionRow::default();
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for CreatorPageMountImp {
    fn constructed(&self) {
        self.parent_constructed();

        let expression = gtk::PropertyExpression::new(
            gtk::StringObject::static_type(),
            None::<gtk::Expression>,
            "string",
        );

        self.mount_type_suggestion.set_expression(expression);

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = gtk::Label::builder()
                .xalign(0.0)
                .use_markup(true)
                // .css_classes(["background"])
                .build();
            item.set_child(Some(&row));
        });

        factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let data = item.item().and_downcast::<gtk::StringObject>().unwrap();

            let child = item.child().and_downcast::<gtk::Label>().unwrap();
            child.set_label(&data.string());
        });

        self.mount_type_suggestion.set_factory(Some(&factory));

        self.obj().connect_showing(|page| {
            let page = page.clone();

            if !page.imp().file_system_names.borrow().is_empty() {
                return;
            }

            glib::spawn_future_local(async move {
                let Ok(file_system_names) = runtime()
                    .block_on(async move { fetch_filesystem_names().await })
                    .inspect_err(|err| warn!("Fetch File System Names Errors {}", err))
                else {
                    return;
                };
                let vec: Vec<&str> = file_system_names.iter().map(|s| s.as_str()).collect();
                let string_list = gtk::StringList::new(&vec);

                page.imp()
                    .mount_type_suggestion
                    .set_model(Some(&string_list));

                page.imp().file_system_names.replace(file_system_names);
            });
        });
    }
}

impl CreatorPageMountImp {}

#[gtk::template_callbacks]
impl CreatorPageMountImp {
    #[template_callback]
    fn environment_add_clicked(&self, _button: gtk::Button) {}
    #[template_callback]
    fn exec_start_dialog_clicked(&self, _button: gtk::Button) {}
}

impl CreatorPageMountImp {
    pub(super) fn update_view(&self, page: &UnitFileCreatorPage) {
        self.fill_data();
        let data = self.file_data.borrow();
        page.update_view(&data);
    }

    pub(super) fn file_content(&self) -> String {
        self.fill_data();
        self.file_data.borrow().to_file()
    }

    fn fill_data(&self) {
        let mut file_data = self.file_data.borrow_mut();

        //        file_data.set_description(self.description_entry.text());

        file_data.sort();
    }

    pub fn update_from_file_content(&self, content: &str) {
        let Some(data) = UnitFileData::from_content(content) else {
            return;
        };

        self.file_data.replace(data);
    }
}

impl WidgetImpl for CreatorPageMountImp {}

impl NavigationPageImpl for CreatorPageMountImp {}

async fn fetch_filesystem_names() -> Result<BTreeSet<String>, SystemdErrors> {
    let mut file_systems_names = BTreeSet::new();
    fetch_kernel_filesystem(&mut file_systems_names).await?;
    fetch_module_filesystem(&mut file_systems_names).await?;

    Ok(file_systems_names)
}

async fn fetch_kernel_filesystem(
    file_systems_names: &mut BTreeSet<String>,
) -> Result<(), SystemdErrors> {
    let file_path = "/proc/filesystems";

    let file = File::open(file_path).await?;
    let reader = BufReader::new(file);

    let mut lines = reader.lines(); // Iterates over lines efficiently without loading the whole file into RAM

    let re = Regex::new(r"(\w*)\t(\w*)").unwrap();

    while let Some(line) = lines.next_line().await? {
        if let Some(cap) = re.captures(&line) {
            // debug!("cap {} fs {}", &cap[1], &cap[2]);
            file_systems_names.insert(cap[2].to_owned());
        } else {
            warn!("Not capture")
        };
        // println!("{}", line);
    }

    Ok(())
}

async fn fetch_module_filesystem(
    file_systems_names: &mut BTreeSet<String>,
) -> Result<(), SystemdErrors> {
    let mut c = commander(["uname", "-r"], None);
    let output = c.output().await.expect("Failed to execute command");

    let kernel_release = String::from_utf8_lossy(&output.stdout);
    let kernel_release = kernel_release.trim();

    let dir_path = format!("/lib/modules/{}/kernel/fs", kernel_release);

    debug!("dir_path {dir_path}");

    let mut rd = fs::read_dir(dir_path).await?;

    while let Some(entry) = rd.next_entry().await? {
        let s = entry.file_name();
        let name = s.to_string_lossy().into_owned();
        // debug!("s {s}");
        file_systems_names.insert(name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use systemd::errors::SystemdErrors;
    use tracing::info;

    #[tokio::test]
    async fn test_kernel_filesystem() -> Result<(), SystemdErrors> {
        test_base::init_logs();

        let mut file_systems_names = BTreeSet::new();
        fetch_kernel_filesystem(&mut file_systems_names).await?;

        info!("{:?}", file_systems_names);
        Ok(())
    }

    #[tokio::test]
    async fn test_module_filesystem() -> Result<(), SystemdErrors> {
        test_base::init_logs();

        let mut file_systems_names = BTreeSet::new();
        fetch_module_filesystem(&mut file_systems_names).await?;
        info!("{:?}", file_systems_names);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_file_system_names() -> Result<(), SystemdErrors> {
        test_base::init_logs();

        let file_systems_names = fetch_filesystem_names().await?;
        info!("{:?}", file_systems_names);

        Ok(())
    }
}
