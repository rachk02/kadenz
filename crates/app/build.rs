fn main() {
    slint_build::compile("ui/fenetre-principale.slint").expect("compilation de l'interface Slint");

    // Icône de kadenz.exe (docs/marque) : ressource Windows intégrée à l'exécutable,
    // visible dans l'Explorateur, sur le bureau et dans la barre des tâches.
    println!("cargo:rerun-if-changed=kadenz.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut ressource = winresource::WindowsResource::new();
        ressource.set_icon("kadenz.ico");
        ressource
            .compile()
            .expect("compilation de la ressource Windows (icône)");
    }
}
