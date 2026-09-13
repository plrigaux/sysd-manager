glib::wrapper! {
    pub struct StandardOutput(ObjectSubclass<imp::StandardOutputImp>);
}

impl StandardOutput {
    pub fn new(list: &str) -> Self {
        let this_object: Self = glib::Object::builder().build();
        this_object.set_string(list);
        this_object
    }
}

pub fn outputs() -> gio::ListStore {
    let list = gio::ListStore::new::<StandardOutput>();

    let output_file_descriptor = output_file_descriptor();

    for fd in output_file_descriptor {
        let out = StandardOutput::new(fd);
        list.append(&out);
    }

    list
}

pub fn output_file_descriptor() -> [&'static str; 12] {
    [
        "inherit",
        "null",
        "tty",
        "journal",
        "kmsg",
        "journal+console",
        "kmsg+console",
        "file:path",
        "append:path",
        "truncate:path",
        "socket",
        "fd:name",
    ]
}

mod imp {
    use crate::gtk::prelude::ObjectExt;
    use crate::gtk::subclass::prelude::DerivedObjectProperties;
    use glib::subclass::{object::ObjectImpl, types::ObjectSubclass};
    use std::cell::RefCell;

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::StandardOutput)]
    pub struct StandardOutputImp {
        #[property(get=Self::label, name="label")]
        #[property(get=Self::text, name="text")]
        #[property(get, set)]
        pub(super) string: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StandardOutputImp {
        const NAME: &'static str = "StandardOutput";
        type Type = super::StandardOutput;
        type ParentType = glib::Object;
        fn new() -> Self {
            Default::default()
        }
    }

    impl StandardOutputImp {
        pub(super) fn label(&self) -> String {
            if let Some((p, s)) = self.string.borrow().split_once(':') {
                format!("{p}:<i>{s}</i>")
            } else {
                self.string.borrow().clone()
            }
        }

        pub(super) fn text(&self) -> String {
            if let Some((p, _)) = self.string.borrow().split_once(':') {
                format!("{p}:")
            } else {
                self.string.borrow().clone()
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for StandardOutputImp {}
}
