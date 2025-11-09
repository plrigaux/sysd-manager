use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
    str::FromStr,
};

use adw::{prelude::*, subclass::window::AdwWindowImpl};
use gettextrs::pgettext;
use std::fmt::Debug;

use gtk::{
    glib::{self},
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};

use log::{debug, error, info, warn};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    consts::{CLASS_WARNING, FLAT},
    systemd::enums::{
        ActiveState, EnablementStatus, LoadState, NumMatchType, Preset, StrMatchType,
        UnitDBusLevel, UnitType,
    },
    upgrade,
    widget::unit_list::{
        COL_ID_UNIT, UnitListPanel,
        filter::{
            UnitListFilterWindow,
            unit_prop_filter::{
                FilterBool, FilterNum, FilterText, UnitPropertyFilter, UnitPropertyFilterType,
                get_filter_element, get_filter_element_mut,
            },
        },
    },
};

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_list_filter.ui")]
#[properties(wrapper_type = super::UnitListFilterWindow)]
pub struct UnitListFilterWindowImp {
    #[template_child]
    filter_stack: TemplateChild<adw::ViewStack>,

    #[template_child]
    filter_navigation_container: TemplateChild<gtk::Box>,

    #[property(get, set, nullable, default = None)]
    selected: RefCell<Option<String>>,

    filter_widgets: RefCell<Vec<Vec<FilterWidget>>>,

    pub(super) unit_list_panel: OnceCell<UnitListPanel>,
}

#[gtk::template_callbacks]
impl UnitListFilterWindowImp {
    #[template_callback]
    fn clear_all_filters_button_clicked(&self, _button: gtk::Button) {
        let unit_list_panel = self
            .unit_list_panel
            .get()
            .expect("unit_list_panel in filter dialog not None");

        unit_list_panel.clear_filters();

        let selection_model = self.filter_stack.pages();
        let list: gio::ListModel = selection_model.into();
        let nb = list.n_items();

        info!("Clean all the {nb} filters");

        for filter_widgets_list in self.filter_widgets.borrow().iter() {
            for filter_widget in filter_widgets_list {
                filter_widget.clear();
            }
        }
    }
}

impl UnitListFilterWindowImp {
    pub(super) fn get_filter(&self) {
        let unit_list_panel = self
            .unit_list_panel
            .get()
            .expect("unit_list_panel in filter dialog not None");

        let binding = self.selected.borrow();
        let selected = binding.as_ref();

        /*

        se and i'll finally end on this okay marketing and innovation are basically

        the same thing now let me explain okay there are two ways you can create value in the marketplace you can either find out what

        people want and work out a really clever way to make it or you can work out what you can make and find a really clever
        way to make people want it. and the money you make is indistinguishable regardless of the direction of travel of that process so
        it isn't necessary to introduce a new product to perform r d
        one other way of doing r d is taking an existing product and presenting it or pricing it or positioning it or framing
        it in a completely different way psychological arbitrage is where quite a
        lot of money is made today there are psychological solutions out there that could save a fortune if you
        want people to get an electric car we currently subsidize electric cars ver
                 */

        let mut filter_widgets: Vec<Vec<FilterWidget>> = vec![];
        // for (name, key, _num_id, _) in &*UNIT_LIST_COLUMNS

        for unit_prop_selection in unit_list_panel.default_displayed_columns().iter().chain(
            unit_list_panel
                .current_columns()
                .iter()
                .filter(|col| col.is_custom()),
        ) {
            let Some(id) = unit_prop_selection.id() else {
                warn!("Column with no id");
                continue;
            };

            let key = id.to_string();
            let name = unit_prop_selection
                .title()
                .map_or(key.clone(), |title| title.to_string());

            let prop_type = unit_prop_selection.prop_type();
            let Some(filter_assessor) = unit_list_panel.lazy_get_filter_assessor(&key, prop_type)
            else {
                warn!("No filter for key {key}");
                continue;
            };

            let (widget, filter_widget): (gtk::Box, Vec<FilterWidget>) = match key.as_str() {
                COL_ID_UNIT => common_text_filter(&filter_assessor),
                "sysdm-bus" => build_bus_level_filter(&filter_assessor),
                "sysdm-type" => build_type_filter(&filter_assessor),
                "sysdm-state" => build_enablement_filter(&filter_assessor),
                "sysdm-preset" => build_preset_filter(&filter_assessor),
                "sysdm-load" => build_load_filter(&filter_assessor),
                "sysdm-active" => build_active_state_filter(&filter_assessor),
                "sysdm-sub" => super::substate::sub_state_filter(&filter_assessor),
                "sysdm-description" => common_text_filter(&filter_assessor),

                _ => match filter_assessor.borrow().ftype() {
                    UnitPropertyFilterType::Text => common_text_filter(&filter_assessor),
                    UnitPropertyFilterType::NumU64 => common_num_filter::<u64>(&filter_assessor),
                    UnitPropertyFilterType::NumI64 => common_num_filter::<i64>(&filter_assessor),
                    UnitPropertyFilterType::NumU32 => common_num_filter::<u32>(&filter_assessor),
                    UnitPropertyFilterType::NumI32 => common_num_filter::<i32>(&filter_assessor),
                    UnitPropertyFilterType::NumU16 => common_num_filter::<u16>(&filter_assessor),
                    UnitPropertyFilterType::Bool => common_bool_filter(&filter_assessor),
                    UnitPropertyFilterType::Element => {
                        error!("Key {key}");
                        let w = gtk::Box::new(gtk::Orientation::Vertical, 0);
                        w.append(&gtk::Label::new(Some(&name)));
                        (w, vec![])
                    }
                },
            };

            filter_widgets.push(filter_widget);

            let _stack_page = self.filter_stack.add_titled(&widget, Some(&key), &name);

            let button_content = adw::ButtonContent::builder()
                .icon_name("empty-icon")
                .label(name)
                .halign(gtk::Align::Start)
                .css_classes(["nav"])
                .build();

            let mut filter_container_binding = filter_assessor.as_ref().borrow_mut();
            let is_empty = filter_container_binding.is_empty();
            button_content.set_icon_name(icon_name(is_empty));
            {
                let button_content = button_content.downgrade();
                let lambda = move |is_empty: bool| {
                    let button_content = upgrade!(button_content);
                    let icon_name = icon_name(is_empty);
                    button_content.set_icon_name(icon_name);
                };

                filter_container_binding.set_on_change(Box::new(lambda));
            }

            let button: gtk::Button = gtk::Button::builder()
                .child(&button_content)
                .css_classes([FLAT])
                .build();

            if selected.is_some_and(|s| s == &key) {
                button.remove_css_class(FLAT);
            }

            let tooltip_text = match key.as_str() {
                "load" => Some("Reflects whether the unit definition was properly loaded."),
                "active" => {
                    Some("The high-level unit activation state, i.e. generalization of <b>Sub</b>.")
                }
                "sub" => Some("The low-level unit activation state, values depend on unit type."),
                _ => None,
            };

            button.set_tooltip_markup(tooltip_text);
            {
                let filter_stack = self.filter_stack.clone();
                button.connect_clicked(move |button| {
                    set_visible_child_name(&filter_stack, &key);

                    if let Some(parent) = button.parent() {
                        let mut child_o = parent.first_child();

                        while let Some(child) = child_o {
                            child.add_css_class("flat");
                            child_o = child.next_sibling();
                        }
                    }
                    button.remove_css_class("flat");
                });
            }
            self.filter_navigation_container.append(&button);
        }

        self.filter_widgets.replace(filter_widgets);

        let box_pad = gtk::Box::builder().vexpand(true).build();

        self.filter_navigation_container.append(&box_pad);

        if let Some(selected) = selected {
            set_visible_child_name(&self.filter_stack, selected);
        }

        self.obj()
            .bind_property::<adw::ViewStack>(
                "selected",
                self.filter_stack.as_ref(),
                "visible-child-name",
            )
            .bidirectional()
            .build();
    }
}

fn set_visible_child_name(filter_stack: &adw::ViewStack, child_name: &str) {
    filter_stack.set_visible_child_name(child_name);
    let widget = filter_stack.child_by_name(child_name);
    if let Some(widget) = widget {
        grab_focus_on_child_entry(widget.first_child().as_ref());
    }
}

fn grab_focus_on_child_entry(widget: Option<&gtk::Widget>) {
    let Some(widget) = widget else {
        return;
    };

    if let Some(entry) = widget.downcast_ref::<gtk::Entry>() {
        entry.grab_focus();
        return;
    }

    grab_focus_on_child_entry(widget.first_child().as_ref());

    grab_focus_on_child_entry(widget.next_sibling().as_ref());
}

fn icon_name(is_empty: bool) -> &'static str {
    if is_empty {
        "empty-icon"
    } else {
        "funnel-symbolic"
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitListFilterWindowImp {
    const NAME: &'static str = "UNIT_LIST_FILTER";
    type Type = UnitListFilterWindow;
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

#[glib::derived_properties]
impl ObjectImpl for UnitListFilterWindowImp {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for UnitListFilterWindowImp {}
impl WindowImpl for UnitListFilterWindowImp {
    // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        /*   self.unit_list_panel
                   .get()
                   .expect("Not None")
                   .clear_unit_list_filter_window_dependancy();
        */
        self.parent_close_request();
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwWindowImpl for UnitListFilterWindowImp {}

pub(crate) fn contain_entry() -> (gtk::Box, gtk::Entry) {
    let merge_box = gtk::Box::builder()
        .css_classes(["linked"])
        .halign(gtk::Align::BaselineFill)
        .build();
    let entry = gtk::Entry::builder()
        .hexpand(true)
        .placeholder_text(pgettext("filter", "Any sub states"))
        .build();
    let button_clear_entry = gtk::Button::builder()
        .icon_name("edit-clear-symbolic")
        .tooltip_text(pgettext("filter", "Clear"))
        .build();
    merge_box.append(&entry);
    merge_box.append(&button_clear_entry);

    let entry_ = entry.clone();
    button_clear_entry.connect_clicked(move |_| {
        entry_.set_text("");
    });

    (merge_box, entry)
}

pub enum FilterWidget {
    Text(gtk::Entry, gtk::DropDown),
    CheckBox(gtk::CheckButton),
    WrapBox(adw::WrapBox),
}

impl FilterWidget {
    fn clear(&self) {
        match self {
            FilterWidget::Text(entry, dropdown) => {
                entry.set_text("");
                dropdown.set_selected(0);
            }

            FilterWidget::CheckBox(check) => {
                check.set_active(false);
            }

            FilterWidget::WrapBox(wrapbox) => {
                while let Some(child) = wrapbox.first_child() {
                    wrapbox.remove(&child);
                }
            }
        }
    }
}

fn common_text_filter(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    let container = create_content_box();

    let merge_box = gtk::Box::builder()
        .css_classes(["linked"])
        .halign(gtk::Align::BaselineFill)
        .build();
    let entry = gtk::Entry::builder().hexpand(true).build();
    let button_clear_entry = gtk::Button::builder()
        .icon_name("edit-clear-symbolic")
        .build();
    merge_box.append(&entry);
    merge_box.append(&button_clear_entry);

    container.append(&merge_box);

    let label = gtk::Label::builder().label("Match type:").build();

    let model_str: Vec<&str> = StrMatchType::iter().map(|x| x.as_str()).collect();
    let model = gtk::StringList::new(&model_str);
    let dropdown = gtk::DropDown::builder()
        .model(&model)
        .halign(gtk::Align::Fill)
        .build();
    let drop_box = gtk::Box::builder()
        .halign(gtk::Align::Fill)
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .build();

    drop_box.append(&label);
    drop_box.append(&dropdown);
    container.append(&drop_box);

    {
        let filter_container = filter_container.borrow();

        //TODO make debug func
        debug!(
            "starting match type {:?} text {:?}",
            filter_container.match_type(),
            filter_container.text()
        );
        entry.set_text(&filter_container.text());
        dropdown.set_selected(filter_container.match_type().position());
    }

    let filter_widget = FilterWidget::Text(entry.clone(), dropdown.clone());
    {
        let filter_container = filter_container.clone();
        entry.connect_changed(move |entry| {
            let text = entry.text();

            let mut binding = filter_container.as_ref().borrow_mut();

            let filter_text = binding
                .as_any_mut()
                .downcast_mut::<FilterText>()
                .expect("downcast_mut to FilterText");

            filter_text.set_filter_elem(&text, true);
        });
    }

    {
        let filter_container: Rc<RefCell<Box<dyn UnitPropertyFilter + 'static>>> =
            filter_container.clone();
        dropdown.connect_selected_item_notify(move |dropdown| {
            let idx = dropdown.selected();
            let match_type: StrMatchType = idx.into();

            debug!("Filter match type idx {idx:?} type {match_type:?}");

            let mut binding = filter_container.as_ref().borrow_mut();

            let filter_text = binding
                .as_any_mut()
                .downcast_mut::<FilterText>()
                .expect("downcast_mut to FilterText");

            filter_text.set_filter_match_type(match_type, true);
        });
    }

    button_clear_entry.connect_clicked(move |_| {
        entry.set_text("");
    });

    (container, vec![filter_widget])
}

#[derive(
    Clone, Copy, Default, Debug, PartialEq, Eq, EnumIter, Hash, glib::Enum, PartialOrd, Ord,
)]
#[enum_type(name = "ActiveState")]
enum BoolFilter {
    True,
    False,
    #[default]
    Unset,
}

impl BoolFilter {
    fn label(&self) -> String {
        match self {
            BoolFilter::True => "True".to_owned(),
            BoolFilter::False => "False".to_owned(),
            BoolFilter::Unset => "<i>Unset</i>".to_owned(),
        }
    }

    fn is_active(&self, value: Option<bool>) -> bool {
        match self {
            BoolFilter::True => value.is_some_and(|v| v),
            BoolFilter::False => value.is_some_and(|v| !v),
            BoolFilter::Unset => value.is_none(),
        }
    }
    fn tooltip_info(&self) -> Option<String> {
        None
    }

    fn get_value(&self) -> Option<bool> {
        match self {
            BoolFilter::True => Some(true),
            BoolFilter::False => Some(false),
            BoolFilter::Unset => None,
        }
    }
}

fn common_bool_filter(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    let container = create_content_box();

    let mut vec = vec![];

    let binding = filter_container.borrow();
    let Some(filter_bool) = binding.as_ref().as_any().downcast_ref::<FilterBool>() else {
        error!("Wrong FilterBool");
        return (gtk::Box::builder().build(), vec);
    };

    for value in BoolFilter::iter() {
        let check = {
            let filter_value = filter_bool.filter_value();
            let active = value.is_active(filter_value);

            let label = gtk::Label::builder()
                .label(value.label())
                .use_markup(true)
                .build();

            gtk::CheckButton::builder()
                .child(&label)
                .active(active)
                .build()
        };

        vec.push(FilterWidget::CheckBox(check.clone()));

        check.set_tooltip_markup(value.tooltip_info().as_deref());

        let filter_container_bool = filter_container.clone();
        check.connect_toggled(move |check_button| {
            let mut binding = filter_container_bool.borrow_mut();
            let Some(filter_bool) = binding.as_any_mut().downcast_mut::<FilterBool>() else {
                error!("Wrong FilterBool Mut");
                return;
            };
            filter_bool.set_filter_elem(value.get_value(), check_button.is_active());
        });

        container.append(&check);
    }

    build_controls(&container);

    (container, vec)
}

fn common_num_filter<T>(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>)
where
    T: Debug + Default + PartialEq + PartialOrd + Copy + FromStr + 'static,
    <T as FromStr>::Err: Debug,
{
    let container = create_content_box();

    let merge_box = gtk::Box::builder()
        .css_classes(["linked"])
        .halign(gtk::Align::BaselineFill)
        .build();
    let entry = gtk::Entry::builder().hexpand(true).build();
    let button_clear_entry = gtk::Button::builder()
        .icon_name("edit-clear-symbolic")
        .build();
    merge_box.append(&entry);
    merge_box.append(&button_clear_entry);

    container.append(&merge_box);

    let label = gtk::Label::builder().label("Match type:").build();

    let model_str: Vec<&str> = NumMatchType::iter().map(|x| x.as_str()).collect();
    let model = gtk::StringList::new(&model_str);

    let dropdown = gtk::DropDown::builder()
        .model(&model)
        .halign(gtk::Align::Fill)
        .build();

    let drop_box = gtk::Box::builder()
        .halign(gtk::Align::Fill)
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .build();

    drop_box.append(&label);
    drop_box.append(&dropdown);
    container.append(&drop_box);

    {
        let filter_container = filter_container.borrow();
        debug!(
            "starting match type {:?} text {:?}",
            filter_container.match_type(),
            filter_container.text()
        );
        entry.set_text(&filter_container.text());
        dropdown.set_selected(filter_container.match_type().position());
    }

    let filter_wiget = FilterWidget::Text(entry.clone(), dropdown.clone());
    {
        let filter_container = filter_container.clone();
        entry.connect_changed(move |entry| {
            let text = entry.text();

            let num_val = if text.is_empty() {
                entry.remove_css_class(CLASS_WARNING);
                None
            } else {
                match text.parse::<T>() {
                    Ok(num_val) => {
                        entry.remove_css_class(CLASS_WARNING);
                        Some(num_val)
                    }
                    Err(e) => {
                        info!("parse error {e:?}");
                        entry.add_css_class(CLASS_WARNING);
                        return;
                    }
                }
            };

            let mut binding = filter_container.as_ref().borrow_mut();

            let filter_text = binding
                .as_any_mut()
                .downcast_mut::<FilterNum<T>>()
                .expect("downcast_mut to FilterText");

            filter_text.set_filter_elem(num_val, true);
        });
    }

    {
        let filter_container: Rc<RefCell<Box<dyn UnitPropertyFilter + 'static>>> =
            filter_container.clone();
        dropdown.connect_selected_item_notify(move |dropdown| {
            let idx = dropdown.selected();
            let match_type: NumMatchType = idx.into();

            debug!("Filter match type idx {idx:?} type {match_type:?}");

            let mut binding = filter_container.as_ref().borrow_mut();

            let filter_text = binding
                .as_any_mut()
                .downcast_mut::<FilterNum<T>>()
                .expect("downcast_mut to FilterText");

            filter_text.set_filter_match_type(match_type, true);
        });
    }

    button_clear_entry.connect_clicked(move |_| {
        entry.set_text("");
    });

    (container, vec![filter_wiget])
}

fn build_type_filter(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    let container = create_content_box();

    //  let filter_elem = Rc::new(RefCell::new(FilterElem::default()));
    for unit_type in UnitType::iter().filter(|x| !matches!(*x, UnitType::Unknown | UnitType::Unit))
    {
        let check = {
            let binding = filter_container.borrow();
            let active = get_filter_element::<UnitType>(binding.as_ref()).contains(&unit_type);

            gtk::CheckButton::builder()
                .label(unit_type.as_str())
                .active(active)
                .build()
        };

        let filter_elem = filter_container.clone();
        check.connect_toggled(move |check_button| {
            //println!("t {} {:?}", check_button.is_active(), unit_type.as_str());
            let mut filter_elem = filter_elem.borrow_mut();
            let filter_element = get_filter_element_mut::<UnitType>(filter_elem.as_mut());
            filter_element.set_filter_elem(unit_type, check_button.is_active());
        });

        container.append(&check);
    }

    build_controls(&container);

    (container, vec![])
}

macro_rules! build_elem_filter {
    ($filter_container:expr, $iter:expr, $value_type:ty) => {{
        let container = create_content_box();

        let mut vec = vec![];
        for value in $iter {
            let check = {
                let binding = $filter_container.borrow();
                let active = get_filter_element::<$value_type>(binding.as_ref()).contains(&value);

                let label = gtk::Label::builder()
                    .label(value.label())
                    .use_markup(true)
                    .build();

                gtk::CheckButton::builder()
                    .child(&label)
                    .active(active)
                    .build()
            };

            vec.push(FilterWidget::CheckBox(check.clone()));

            check.set_tooltip_markup(value.tooltip_info().as_deref());

            let filter_elem = $filter_container.clone();
            check.connect_toggled(move |check_button| {
                let mut filter_elem = filter_elem.borrow_mut();
                let filter_element = get_filter_element_mut::<$value_type>(filter_elem.as_mut());
                filter_element.set_filter_elem(value, check_button.is_active());
            });

            container.append(&check);
        }

        build_controls(&container);

        (container, vec)
    }};
}

fn build_preset_filter(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    build_elem_filter!(filter_container, Preset::iter(), Preset)
}

fn build_bus_level_filter(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    build_elem_filter!(filter_container, UnitDBusLevel::iter(), UnitDBusLevel)
}

pub(crate) fn create_content_box() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .width_request(300)
        .spacing(5)
        .margin_start(5)
        .margin_top(5)
        .margin_end(5)
        .build()
}

fn build_controls(container: &gtk::Box) {
    let controls = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(["linked"])
        .halign(gtk::Align::Center)
        .build();

    let clear_button = gtk::Button::builder()
        .label(pgettext("filter", "Clear"))
        .tooltip_text(pgettext("filter", "Clear filter's selected items"))
        .build();
    {
        let container = container.clone();
        clear_button.connect_clicked(move |_| {
            let mut some_widget = container.first_child();
            while let Some(w) = some_widget.as_ref() {
                let maybe_checkbutton = w.downcast_ref::<gtk::CheckButton>();
                if let Some(check) = maybe_checkbutton {
                    check.set_active(false);
                }

                some_widget = w.next_sibling();
            }
        });
    }

    let inv_button = gtk::Button::builder()
        .label(pgettext("filter", "Invert"))
        .tooltip_text(pgettext("filter", "Invert filter's selected items"))
        .build();
    {
        let container = container.clone();
        inv_button.connect_clicked(move |_| {
            let mut some_widget = container.first_child();

            while let Some(w) = some_widget.as_ref() {
                let maybe_checkbutton = w.downcast_ref::<gtk::CheckButton>();
                if let Some(check) = maybe_checkbutton {
                    check.set_active(!check.is_active());
                }

                some_widget = w.next_sibling();
            }
        });
    }
    controls.append(&clear_button);
    controls.append(&inv_button);

    container.append(&controls);
}

fn build_enablement_filter(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    build_elem_filter!(filter_container, EnablementStatus::iter(), EnablementStatus)
}

fn build_load_filter(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    build_elem_filter!(filter_container, LoadState::iter(), LoadState)
}

fn build_active_state_filter(
    filter_container: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    build_elem_filter!(filter_container, ActiveState::iter(), ActiveState)
}
