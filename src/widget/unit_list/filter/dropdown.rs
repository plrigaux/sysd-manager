use gtk::StringList;

fn drop_down() {
    // let d = gtk::DropDown::new(model, expression);
}

fn create_model() {
    let root = StringList::new(&["x", "b"]);
    gtk::TreeListModel::new(root, true, true, |x| {
        let model = StringList::new(&["a", "b", "c"]);
        Some(model.into())
    });
}
