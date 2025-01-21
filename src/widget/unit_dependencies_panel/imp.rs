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

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashSet},
    rc::Rc,
};

use crate::{
    systemd::{
        self,
        data::UnitInfo,
        enums::{DependencyType, UnitType},
        Dependency,
    },
    widget::{
        app_window::AppWindow,
        menu_button::{ExMenuButton, OnClose},
        unit_info::{
            text_view_hyperlink::{self, LinkActivator},
            writer::{
                HyperLinkType, UnitInfoWriter, SPECIAL_GLYPH_TREE_BRANCH, SPECIAL_GLYPH_TREE_RIGHT,
                SPECIAL_GLYPH_TREE_SPACE, SPECIAL_GLYPH_TREE_VERTICAL,
            },
        },
    },
};
use log::{debug, warn};
use strum::IntoEnumIterator;

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

    #[template_child]
    dependency_types_dropdown: TemplateChild<gtk::DropDown>,

    #[template_child]
    controls_box: TemplateChild<gtk::Box>,

    #[property(get, set=Self::set_visible_on_page)]
    visible_on_page: Cell<bool>,

    #[property(get, set=Self::set_unit)]
    unit: RefCell<Option<UnitInfo>>,

    #[property(get, set)]
    dark: Cell<bool>,

    unit_dependencies_loaded: Cell<bool>,

    pub(super) dependency_type: Cell<DependencyType>,

    plain: Cell<bool>,

    hovering_over_link_tag: Rc<RefCell<Option<gtk::TextTag>>>,

    unit_type_filter: RefCell<HashSet<String>>,
}

#[gtk::template_callbacks]
impl UnitDependenciesPanelImp {
    #[template_callback]
    fn plain_option_toggled(&self, check_button: &gtk::CheckButton) {
        self.plain.set(check_button.is_active());
        self.update_dependencies();
    }
}

impl UnitDependenciesPanelImp {
    pub(crate) fn register(&self, app_window: &AppWindow) {
        {
            let app_window = app_window.clone();

            let activator = LinkActivator::new(Some(app_window));

            text_view_hyperlink::build_textview_link_platform(
                &self.unit_dependencies_textview,
                self.hovering_over_link_tag.clone(),
                activator,
            );
        }
    }

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

    pub(super) fn update_dependencies_filtered(&self, unit_type_filter: &HashSet<String>) {
        self.unit_type_filter.replace(unit_type_filter.clone());
        self.update_dependencies();
    }

    pub(super) fn update_dependencies(&self) {
        let binding = self.unit.borrow();
        let Some(unit_ref) = binding.as_ref() else {
            warn!("No unit file");
            return;
        };

        self.unit_dependencies_loaded.set(true); // maybe wait at the full loaded

        let dep_type = self.dependency_type.get();
        let unit = unit_ref.clone();
        let textview = self.unit_dependencies_textview.clone();
        let stack = self.unit_dependencies_panel_stack.clone();
        let dark = self.dark.get();
        let mut plain = self.plain.get();
        let unit_type_filter = self.unit_type_filter.borrow().clone();

        plain = plain || !unit_type_filter.is_empty();

        glib::spawn_future_local(async move {
            stack.set_visible_child_name("spinner");
            let dependencies =
                gio::spawn_blocking(move || {
                    match systemd::fetch_unit_dependencies(&unit, dep_type, plain) {
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

            let Some(mut dependencies) = dependencies else {
                stack.set_visible_child_name("empty");
                return;
            };

            if !unit_type_filter.is_empty() {
                let mut set = BTreeSet::new();
                for dep in dependencies.children {
                    if let Some((_, unit_type)) = dep.unit_name.rsplit_once('.') {
                        if unit_type_filter.contains(unit_type) {
                            set.insert(dep);
                        }
                    }
                }
                dependencies.children = set;
            }

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
        info_writer.insert_state(dependency.state);
        //info_writer.insert(" ");
        info_writer.insert(spacer);

        let (glyph, child_pading) = if last {
            (SPECIAL_GLYPH_TREE_RIGHT, SPECIAL_GLYPH_TREE_SPACE)
        } else {
            (SPECIAL_GLYPH_TREE_BRANCH, SPECIAL_GLYPH_TREE_VERTICAL)
        };

        info_writer.insert(glyph);
        info_writer.insert(" ");
        info_writer.hyperlink(
            &dependency.unit_name,
            &dependency.unit_name,
            HyperLinkType::Unit,
        );
        info_writer.newline();

        let child_spacer = format!("{spacer}{child_pading}");

        let mut it = dependency.children.iter().peekable();
        while let Some(child) = it.next() {
            let child_last = it.peek().is_none();
            UnitDependenciesPanelImp::display_dependencies(
                info_writer,
                child,
                &child_spacer,
                child_last,
            );
        }
    }

    fn setup_dependency_type_dropdown(&self) {
        let expression = gtk::PropertyExpression::new(
            adw::EnumListItem::static_type(),
            None::<gtk::Expression>,
            "nick",
        );

        self.dependency_types_dropdown
            .set_expression(Some(expression));

        let model = adw::EnumListModel::new(DependencyType::static_type());

        self.dependency_types_dropdown.set_model(Some(&model));

        {
            let dependency_panel = self.obj().clone();
            self.dependency_types_dropdown
                .connect_selected_item_notify(move |dropdown| {
                    let idx = dropdown.selected();
                    let dependency_type: DependencyType = idx.into();

                    debug!(
                        "System Session Values Selected idx {:?} level {:?}",
                        idx, -1
                    );

                    let old = dependency_panel.replace_dependency_type(dependency_type);

                    if old != dependency_type {
                        dependency_panel.update_dependencies()
                    }
                });
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

        self.setup_dependency_type_dropdown();

        let mut filter_button_unit_type = ExMenuButton::new("Type");
        filter_button_unit_type.set_margin_end(5);
        filter_button_unit_type.set_tooltip_text(Some("Filter dependencies by types"));

        for unit_type in UnitType::iter().filter(|x| !matches!(*x, UnitType::Unknown(_))) {
            filter_button_unit_type.add_item(unit_type.to_str());
        }

        self.controls_box.prepend(&filter_button_unit_type);

        let dep = self.obj();
        let on_close = OnClose::new_dep(&dep);
        filter_button_unit_type.set_on_close(on_close);
    }
}

impl WidgetImpl for UnitDependenciesPanelImp {}
impl BoxImpl for UnitDependenciesPanelImp {}
