//! Copies `assets/` next to the compiled binary (e.g. `target/debug/assets`)
//! so the app can find its bundled files at runtime.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets");

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    if !src.exists() {
        return;
    }

    // OUT_DIR is target/<profile>/build/<pkg>-<hash>/out
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let profile_dir = out_dir.ancestors().nth(3).unwrap();
    copy_dir(&src, &profile_dir.join("assets"));
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to);
        } else if needs_copy(&from, &to) {
            fs::copy(&from, &to).unwrap();
        }
    }
}

/// Skip unchanged files so large models aren't re-copied on every build.
fn needs_copy(from: &Path, to: &Path) -> bool {
    let (Ok(src), Ok(dst)) = (fs::metadata(from), fs::metadata(to)) else {
        return true;
    };
    src.len() != dst.len() || src.modified().ok() > dst.modified().ok()
}
