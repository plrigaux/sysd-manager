use super::*;
use crate::widget::creator::{CreateUnitErr, suggestion::SuggestionRow, unit_file::UnitFileData};
use adw::subclass::prelude::*;
use regex::Regex;
use std::{
    cell::{OnceCell, RefCell},
    path::Path,
};
use tracing::info;

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/creator_page_mount.ui")]
#[properties(wrapper_type = super::CreatorPageMount)]
pub struct CreatorPageMountImp {
    pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

    pub(super) file_data: RefCell<UnitFileData>,
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

#[cfg(test)]
mod tests {
    use regex::Regex;
    use systemd::errors::SystemdErrors;
    use tokio::{
        fs::{self, File},
        io::{AsyncBufReadExt, BufReader},
        process::Command,
    };
    use tracing::{info, warn};

    #[tokio::test]
    async fn test_kernel_filesystem() -> Result<(), SystemdErrors> {
        test_base::init_logs();
        let file_path = "/proc/filesystems";
        // let file = File::open("/proc/filesystems").await?;

        let out = fs::read_to_string(file_path).await?;

        println!("filesystems\n{}", out);

        let file = File::open(file_path).await?;
        let reader = BufReader::new(file);

        let mut lines = reader.lines(); // Iterates over lines efficiently without loading the whole file into RAM

        let re = Regex::new(r"(\w*)\t(\w*)").unwrap();

        let mut file_systems_names = Vec::new();
        while let Some(line) = lines.next_line().await? {
            if let Some(cap) = re.captures(&line) {
                info!("cap {} fs {}", &cap[1], &cap[2]);
                file_systems_names.push(cap[2].to_owned());
            } else {
                warn!("Not capture")
            };
            // println!("{}", line);
        }

        file_systems_names.sort();

        info!("{:?}", file_systems_names);
        Ok(())
    }

    #[tokio::test]
    async fn test_module_filesystem() -> Result<(), SystemdErrors> {
        test_base::init_logs();
        let output = Command::new("uname")
            .arg("-r")
            .output()
            .await
            .expect("Failed to execute command");

        let kernel_release = String::from_utf8_lossy(&output.stdout);
        let kernel_release = kernel_release.trim();

        let dir_path = format!("/lib/modules/{}/kernel/fs", kernel_release);

        info!("dir_path {dir_path}");

        let mut rd = fs::read_dir(dir_path).await?;

        let mut file_systems_names = Vec::new();
        while let Some(entry) = rd.next_entry().await? {
            let s = entry.file_name();
            let s = s.to_string_lossy().into_owned();
            info!("s {s}");
            file_systems_names.push(s);
        }

        file_systems_names.sort();
        info!("{:?}", file_systems_names);

        Ok(())
    }
}
