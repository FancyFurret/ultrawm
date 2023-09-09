use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rustc-link-search=/System/Library/Frameworks/");
    println!("cargo:rustc-link-lib=framework=ApplicationServices");
    println!("cargo:rerun-if-changed=wrapper.h");

    let sdk_path = Command::new("xcrun")
        .args(&["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("Failed to get SDK path")
        .stdout;
    let sdk_path = String::from_utf8(sdk_path)
        .expect("Failed to convert to String")
        .trim()
        .to_string();

    println!("sdk_path: {}", sdk_path);
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-F{}/System/Library/Frameworks", sdk_path))
        .allowlist_function("AX.*")
        .allowlist_var("kAX.*")
        .allowlist_type("AX.*")
        .blocklist_type("CF.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
