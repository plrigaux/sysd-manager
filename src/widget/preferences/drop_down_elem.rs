use std::cell::Ref;

use super::data::{KEY_PREF_PREFERED_COLOR_SCHEME, PreferedColorScheme};
use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, translate::IntoGlib};
use sourceview5::prelude::ToValue;
use strum::IntoEnumIterator;

glib::wrapper! {
    pub struct DropDownItem(ObjectSubclass<imp::DropDownItemImpl>);
}

impl Default for DropDownItem {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DropDownItem {
    pub fn new(icon: &str, text: &str) -> Self {
        let o: DropDownItem = glib::Object::new();
        o.imp().assign(icon, text);
        o
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::DropDownItem)]
    pub struct DropDownItemImpl {
        #[property(get, set)]
        pub text: RefCell<String>,
        #[property(get, set)]
        pub icon: RefCell<String>,
    }

    impl DropDownItemImpl {
        pub(super) fn assign(&self, icon: &str, text: &str) {
            self.text.replace(text.to_owned());
            self.icon.replace(icon.to_owned());
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DropDownItemImpl {
        const NAME: &'static str = "DropDownItem";
        type Type = super::DropDownItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DropDownItemImpl {}
}

pub(super) fn build_pane_orientation_selector(app_orientation: &adw::ComboRow) {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();

    /*     store.append(&DropDownItem::new("3", "text"));
    store.append(&DropDownItem::new("sdfgs", "text1"));
    store.append(&DropDownItem::new("sdf", "text2"));
    store.append(&DropDownItem::new("sdgf", "text3"));

    app_orientation.set_model(Some(&store)); */

    /*     let boxed = glib::BoxedAnyObject::new(Author {
        name: String::from("GLibAuthor"),
        subscribers: 1000,
    });

    store.append(&boxed);

    let boxed = glib::BoxedAnyObject::new(Author {
        name: String::from("Test"),
        subscribers: 2,
    });

    store.append(&boxed); */

    app_orientation.set_model(Some(&store));
    /*
    let expression =
        gtk::PropertyExpression::new(DropDownItem::static_type(), None::<gtk::Expression>, "text");

    let expression = gtk::ConstantExpression::new("Pizza"); */

    /*     let closure = glib::RustClosure::new(|values| {
        let x = values[0].get::<glib::BoxedAnyObject>().unwrap();
        let x: std::cell::Ref<'_, Author> = x.borrow();
        let x = &x.name;
        Some(format!("{x} du coup").to_value())
    });

    let v: Vec<gtk::Expression> = Vec::new();
    let expression = gtk::ClosureExpression::new::<String>(v, closure); */

    //app_orientation.set_expression(Some(&expression));
}

pub(super) fn build_prefered_color_scheme(
    prefered_color_scheme: &adw::ComboRow,
    settings: &gio::Settings,
) {
    let model = gio::ListStore::new::<glib::BoxedAnyObject>();

    for color_scheme in PreferedColorScheme::iter() {
        let boxed = glib::BoxedAnyObject::new(color_scheme);
        model.append(&boxed);
    }

    prefered_color_scheme.set_model(Some(&model));

    let expression = gtk::ClosureExpression::new::<String>(
        Vec::<gtk::Expression>::new(),
        glib::RustClosure::new(|values| {
            let boxed = values[0].get::<glib::BoxedAnyObject>().unwrap();
            let color_ref: Ref<'_, PreferedColorScheme> = boxed.borrow();
            Some(color_ref.text().to_value())
        }),
    );

    prefered_color_scheme.set_expression(Some(expression));

    prefered_color_scheme.connect_selected_item_notify(|combo_box| {
        let selected_item = combo_box.selected_item();

        let Some(color_scheme) = selected_item else {
            return;
        };

        let binding = color_scheme
            .downcast::<glib::BoxedAnyObject>()
            .expect("Needs to be BoxedAnyObject");
        let color_scheme: Ref<'_, PreferedColorScheme> = binding.borrow();

        let manager = adw::StyleManager::default();
        manager.set_color_scheme(color_scheme.color_scheme());
    });

    settings
        .bind(
            KEY_PREF_PREFERED_COLOR_SCHEME,
            prefered_color_scheme,
            "selected",
        )
        .mapping(|a, _| {
            let v = a.get::<i32>().unwrap();

            let drop_own_index = if let Some((drop_own_index, _)) = PreferedColorScheme::iter()
                .enumerate()
                .find(|(_idx, color_scheme)| v == color_scheme.color_scheme().into_glib())
            {
                drop_own_index as i32
            } else {
                0
            };

            let value = drop_own_index.to_value();
            Some(value)
        })
        .set_mapping(|value, _| {
            let drop_own_index = value.get::<u32>().unwrap();
            let mut color_scheme_selected = PreferedColorScheme::Default;
            for (idx, color_scheme) in PreferedColorScheme::iter().enumerate() {
                color_scheme_selected = color_scheme;
                if drop_own_index == idx as u32 {
                    break;
                }
            }

            let variant = color_scheme_selected
                .color_scheme()
                .into_glib()
                .to_variant();

            Some(variant)
        })
        .build();
}
