fn main() {
    // Tauri validates `bundle.resources` at build time. The llama-server
    // runtime dirs are gitignored and populated by
    // `scripts/packaging/fetch-llama-server.ps1`, so a fresh clone has no
    // `binaries/*` dirs and `cargo test` would fail with
    // "resource path `binaries\\cpu` doesn't exist". Ensure the reserved
    // slots exist (empty is fine for unit tests and CI); packaging still
    // requires the fetch script to populate them before a release build.
    for dir in ["binaries/vulkan", "binaries/cpu", "binaries/cuda"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        if let Err(e) = std::fs::create_dir_all(&path) {
            eprintln!("warning: could not create {}: {e}", path.display());
        }
    }
    tauri_build::build()
}
