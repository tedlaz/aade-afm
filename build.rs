fn main() {
    // icon + manifest (Common Controls v6, DPI)
    embed_resource::compile("assets/app.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
