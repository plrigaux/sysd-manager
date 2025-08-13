use gtk::{
    glib::{self},
    prelude::*,
    subclass::prelude::*,
};
pub fn drop_down() -> gtk::DropDown {
    let states = sub_states_tree();

    let list_model_containing_list_models = gio::ListStore::new::<gio::ListStore>();
    for (category, cat_sub_states) in states {
        let list_model = gio::ListStore::new::<SubState>();
        for cat_sub_state in cat_sub_states {
            let sub_state_obj = SubState::new(category, cat_sub_state);
            list_model.append(&sub_state_obj);
        }

        list_model_containing_list_models.append(&list_model);
    }

    let flat_list_model = gtk::FlattenListModel::new(Some(list_model_containing_list_models));

    let expression = gtk::PropertyExpression::new(
        SubState::static_type(),
        None::<gtk::Expression>,
        "sub_state",
    );

    let dropdown = gtk::DropDown::new(Some(flat_list_model), Some(expression));
    dropdown.set_enable_search(true);
    dropdown.set_search_match_mode(gtk::StringFilterMatchMode::Substring);

    let header_factory = gtk::SignalListItemFactory::new();

    header_factory.connect_setup(|_factory, item| {
        let item = item.downcast_ref::<gtk::ListHeader>().unwrap();
        let child = gtk::Label::builder()
            .selectable(true)
            .xalign(0.0)
            .use_markup(true)
            .margin_top(10)
            .margin_bottom(10)
            .build();
        item.set_child(Some(&child));
    });

    header_factory.connect_bind(move |_factory, item| {
        let list_header = item.downcast_ref::<gtk::ListHeader>().unwrap();
        let Some(item) = list_header.item() else {
            return;
        };

        let item = item.downcast_ref::<SubState>().unwrap();

        let Some(widget) = list_header.child() else {
            return;
        };

        let label = widget.downcast_ref::<gtk::Label>().unwrap();

        if item.category() != "" {
            label.set_label(&format!("<big><b>{}</b></big>", item.category()));
        }
    });

    dropdown.set_header_factory(Some(&header_factory));

    dropdown.set_selected(gtk::INVALID_LIST_POSITION);

    dropdown
}

/// from systemctl --state=help
fn sub_states_tree() -> Vec<(&'static str, Vec<&'static str>)> {
    // let d = gtk::DropDown::new(model, expression);

    let states = vec![
        //FIXME workaround to bypass gtk drpdown autoselection
        ("", vec![""]),
        (
            "Unit load",
            vec![
                "stub",
                "loaded",
                "not-found",
                "bad-setting",
                "error",
                "merged",
                "masked",
            ],
        ),
        (
            "Unit active",
            vec![
                "active",
                "reloading",
                "inactive",
                "failed",
                "activating",
                "deactivating",
                "maintenance",
                "refreshing",
            ],
        ),
        (
            "Unit file",
            vec![
                "enabled",
                "enabled-runtime",
                "linked",
                "linked-runtime",
                "alias",
                "masked",
                "masked-runtime",
                "static",
                "disabled",
                "indirect",
                "generated",
                "transient",
                "bad",
            ],
        ),
        (
            "Automount unit",
            vec!["dead", "waiting", "running", "failed"],
        ),
        ("Device unit", vec!["dead", "tentative", "plugged"]),
        (
            "Mount unit",
            vec![
                "dead",
                "mounting",
                "mounting-done",
                "mounted",
                "remounting",
                "unmounting",
                "remounting-sigterm",
                "remounting-sigkill",
                "unmounting-sigterm",
                "unmounting-sigkill",
                "failed",
                "cleaning",
            ],
        ),
        ("Path unit", vec!["dead", "waiting", "running", "failed"]),
        (
            "Scope unit",
            vec![
                "dead",
                "start-chown",
                "running",
                "abandoned",
                "stop-sigterm",
                "stop-sigkill",
                "failed",
            ],
        ),
        (
            "Service unit",
            vec![
                "dead",
                "condition",
                "start-pre",
                "start",
                "start-post",
                "running",
                "exited",
                "reload",
                "reload-signal",
                "reload-notify",
                "mounting",
                "stop",
                "stop-watchdog",
                "stop-sigterm",
                "stop-sigkill",
                "stop-post",
                "final-watchdog",
                "final-sigterm",
                "final-sigkill",
                "failed",
                "dead-before-auto-restart",
                "failed-before-auto-restart",
                "dead-resources-pinned",
                "auto-restart",
                "auto-restart-queued",
                "cleaning",
            ],
        ),
        ("Slice unit", vec!["dead", "active"]),
        (
            "Socket unit",
            vec![
                "dead",
                "start-pre",
                "start-chown",
                "start-post",
                "listening",
                "running",
                "stop-pre",
                "stop-pre-sigterm",
                "stop-pre-sigkill",
                "stop-post",
                "final-sigterm",
                "final-sigkill",
                "failed",
                "cleaning",
            ],
        ),
        (
            "Socket unit",
            vec![
                "dead",
                "start-pre",
                "start-chown",
                "start-post",
                "listening",
                "running",
                "stop-pre",
                "stop-pre-sigterm",
                "stop-pre-sigkill",
                "stop-post",
                "final-sigterm",
                "final-sigkill",
                "failed",
                "cleaning",
            ],
        ),
        (
            "Swap unit",
            vec![
                "dead",
                "activating",
                "activating-done",
                "active",
                "deactivating",
                "deactivating-sigterm",
                "deactivating-sigkill",
                "failed",
                "cleaning",
            ],
        ),
        ("Target unit", vec!["dead", "active"]),
        (
            "Timer unit",
            vec!["dead", "waiting", "running", "elapsed", "failed"],
        ),
    ];

    states
}

glib::wrapper! {
    pub struct SubState(ObjectSubclass<imp::SubStateImpl>);
}

impl Default for SubState {
    fn default() -> Self {
        let this_object: Self = glib::Object::new();

        this_object
    }
}

impl SubState {
    fn new(category: &str, sub_state: &str) -> Self {
        let this_object: Self = glib::Object::new();
        this_object.imp().init(category, sub_state);
        this_object
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::SubState)]
    pub struct SubStateImpl {
        #[property(get)]
        pub(super) category: RefCell<String>,

        #[property(get)]
        pub(super) sub_state: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SubStateImpl {
        const NAME: &'static str = "SubState";
        type Type = super::SubState;

        fn new() -> Self {
            Default::default()
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SubStateImpl {}

    impl SubStateImpl {
        pub(super) fn init(&self, category: &str, sub_state: &str) {
            self.category.replace(category.to_string());
            self.sub_state.replace(sub_state.to_string());
        }
    }
}
