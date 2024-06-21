fn main() {
    glib_build_tools::compile_resources(
        &["data"],
        "data/resources/resources.gresource.xml",
        "sysd-manager.gresource",
    );
}