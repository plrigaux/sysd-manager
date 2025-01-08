use gtk::{
    gio,
    glib::{self},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
    TemplateChild,
};

use std::cell::{Cell, RefCell};

use log::{debug, warn};

use crate::{
    systemd::{self, data::UnitInfo, enums::DependencyType, Dependency},
    widget::unit_info::writer::{
        UnitInfoWriter, SPECIAL_GLYPH_TREE_BRANCH, SPECIAL_GLYPH_TREE_RIGHT,
        SPECIAL_GLYPH_TREE_SPACE, SPECIAL_GLYPH_TREE_VERTICAL,
    },
};

/* const PANEL_EMPTY: &str = "empty";
const PANEL_JOURNAL: &str = "journal";
const PANEL_SPINNER: &str = "spinner";
 */
#[derive(Default, glib::Properties, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_dependencies_panel.ui")]
#[properties(wrapper_type = super::UnitDependenciesPanel)]
pub struct UnitDependenciesPanelImp {
    #[template_child]
    unit_dependencies_panel_stack: TemplateChild<gtk::Stack>,

    #[template_child]
    unit_dependencies_textview: TemplateChild<gtk::TextView>,

    #[property(get, set=Self::set_visible_on_page)]
    visible_on_page: Cell<bool>,

    #[property(get, set=Self::set_unit)]
    unit: RefCell<Option<UnitInfo>>,

    #[property(get, set)]
    dark: Cell<bool>,

    unit_dependencies_loaded: Cell<bool>,
}

#[gtk::template_callbacks]
impl UnitDependenciesPanelImp {
    fn set_visible_on_page(&self, value: bool) {
        debug!("set_visible_on_page val {value}");
        self.visible_on_page.set(value);

        if self.visible_on_page.get()
            && !self.unit_dependencies_loaded.get()
            && self.unit.borrow().is_some()
        {
            self.update_dependencies()
        }
    }

    fn set_unit(&self, unit: &UnitInfo) {
        let old_unit = self.unit.replace(Some(unit.clone()));
        if let Some(old_unit) = old_unit {
            if old_unit.primary() != unit.primary() {
                self.unit_dependencies_loaded.set(false)
            }
        }

        if self.visible_on_page.get() {
            self.update_dependencies()
        }
    }

    fn update_dependencies(&self) {
        let binding = self.unit.borrow();
        let Some(unit_ref) = binding.as_ref() else {
            warn!("No unit file");
            return;
        };

        self.unit_dependencies_loaded.set(true); // maybe wait at the full loaded

        let dep_type = DependencyType::Forward;
        let unit = unit_ref.clone();
        let textview = self.unit_dependencies_textview.clone();
        let stack = self.unit_dependencies_panel_stack.clone();
        let dark = self.dark.get();

        glib::spawn_future_local(async move {
            stack.set_visible_child_name("spinner");
            let dependencies =
                gio::spawn_blocking(move || {
                    match systemd::fetch_unit_dependencies(&unit, dep_type) {
                        Ok(dep) => Some(dep),
                        Err(error) => {
                            warn!(
                                "Fetching {:?} dependencies error {:?}",
                                unit.primary(),
                                error
                            );
                            None
                        }
                    }
                })
                .await
                .expect("Task needs to finish successfully.");

            let Some(dependencies) = dependencies else {
                stack.set_visible_child_name("empty");
                return;
            };

            let buf = textview.buffer();
            buf.set_text(""); // clear text

            let start_iter = buf.start_iter();

            let mut info_writer = UnitInfoWriter::new(buf, start_iter, dark);

            info_writer.insertln(&dependencies.unit_name);

            let spacer = String::from(SPECIAL_GLYPH_TREE_SPACE);
            let mut it = dependencies.children.iter().peekable();

            while let Some(child) = it.next() {
                UnitDependenciesPanelImp::display_dependencies(
                    &mut info_writer,
                    child,
                    &spacer,
                    it.peek().is_none(),
                );
            }

            stack.set_visible_child_name("dependencies");
        });
    }

    fn display_dependencies(
        info_writer: &mut UnitInfoWriter,
        dependency: &Dependency,

        spacer: &str,
        last: bool,
    ) {
        let state_glyph = dependency.state.glyph();

        let gl = format!("{state_glyph} ");

        match dependency.state {
            systemd::enums::ActiveState::Active
            | systemd::enums::ActiveState::Reloading
            | systemd::enums::ActiveState::Activating
            | systemd::enums::ActiveState::Refreshing => info_writer.insert_active(&gl),

            systemd::enums::ActiveState::Inactive | systemd::enums::ActiveState::Deactivating => {
                info_writer.insert(&gl);
            }
            _ => info_writer.insert_red(&gl),
        }

        info_writer.insert(&spacer);

        let (glyph, child_pading) = if last {
            (SPECIAL_GLYPH_TREE_RIGHT, SPECIAL_GLYPH_TREE_SPACE)
        } else {
            (SPECIAL_GLYPH_TREE_BRANCH, SPECIAL_GLYPH_TREE_VERTICAL)
        };

        info_writer.insert(glyph);
        info_writer.insert(&dependency.unit_name);
        info_writer.newline();

        let child_spacer = format!("{spacer}{child_pading}");

        let mut it = dependency.children.iter().peekable();
        while let Some(child) = it.next() {
            let child_last = it.peek().is_none();
            UnitDependenciesPanelImp::display_dependencies(
                info_writer,
                &child,
                &child_spacer,
                child_last,
            );
        }
    }
}
// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitDependenciesPanelImp {
    const NAME: &'static str = "UnitDependenciesPanel";
    type Type = super::UnitDependenciesPanel;
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

#[glib::derived_properties]
impl ObjectImpl for UnitDependenciesPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.unit_dependencies_loaded.set(false);
    }
}

impl WidgetImpl for UnitDependenciesPanelImp {}
impl BoxImpl for UnitDependenciesPanelImp {}
