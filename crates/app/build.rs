fn main() {
    // Traductions de l'interface (src/traduction.rs) : lang/<code>/LC_MESSAGES/kadenz.po,
    // embarquées dans l'exécutable. Le français est la langue source, sans contexte
    // de traduction : un même texte a la même traduction dans tous les écrans.
    println!("cargo:rerun-if-changed=lang");
    let configuration = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/fenetre-principale.slint", configuration)
        .expect("compilation de l'interface Slint");

    // Icône de kadenz.exe (docs/marque) : ressource Windows intégrée à l'exécutable,
    // visible dans l'Explorateur, sur le bureau et dans la barre des tâches.
    println!("cargo:rerun-if-changed=kadenz.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut ressource = winresource::WindowsResource::new();
        ressource.set_icon("kadenz.ico");
        // Éditeur affiché dans les propriétés du fichier (décision de
        // l'utilisateur du 2026-11-11), comme dans l'installateur.
        ressource.set("CompanyName", "Mcarre");
        ressource.set("ProductName", "Kadenz");
        ressource.set("FileDescription", "Kadenz, gestion commerciale");
        ressource
            .compile()
            .expect("compilation de la ressource Windows (icône)");
    }
}
