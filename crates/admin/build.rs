fn main() {
    // Mêmes traductions que l'application (crates/app/lang) : une seule source
    // pour les textes communs, dont l'assistant de l'entreprise.
    println!("cargo:rerun-if-changed=../app/lang");
    // Slint cherche lang/<code>/LC_MESSAGES/<domaine>.po, le domaine étant le nom
    // du paquet lu dans CARGO_PKG_NAME : celui de l'application, « kadenz ».
    // SAFETY : le script de construction n'a qu'un fil d'exécution.
    unsafe { std::env::set_var("CARGO_PKG_NAME", "kadenz") };
    let configuration = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("../app/lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/admin.slint", configuration)
        .expect("compilation de l'interface Slint");

    // Icône de Kadenz et manifeste : la fenêtre demande les droits
    // d'administrateur, car le dossier du serveur est réservé à SYSTEM et aux
    // Administrateurs, et la clé de la base est protégée au niveau de la machine.
    println!("cargo:rerun-if-changed=../app/kadenz.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut ressource = winresource::WindowsResource::new();
        ressource.set_icon("../app/kadenz.ico");
        ressource.set("CompanyName", "Mcarre");
        ressource.set("ProductName", "Kadenz");
        ressource.set("FileDescription", "Kadenz, administration du serveur");
        ressource.set_manifest(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#,
        );
        ressource
            .compile()
            .expect("compilation de la ressource Windows (icône et manifeste)");
    }
}
