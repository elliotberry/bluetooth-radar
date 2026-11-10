fn main() {
    // The icons are embedded into the binary at compile time; without this,
    // cargo does not rebuild when only an icon file changes.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
