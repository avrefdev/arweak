fn main() {
    let manifest = include_str!("arweak.exe.manifest");
    let attrs = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(manifest));
    tauri_build::try_build(attrs).expect("tauri build");
}
