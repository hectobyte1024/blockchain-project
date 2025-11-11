use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to look for the C++ library
    let cpp_core_path = "../../cpp-core";
    let current_dir = env::current_dir().unwrap();
    println!("cargo:rustc-link-search=native={}", current_dir.display());
    println!("cargo:rustc-link-lib=static=blockchain_consensus");
    
    // Link OpenSSL for SHA-256
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
    
    // Link C++ standard library
    println!("cargo:rustc-link-lib=stdc++");
    
    // Rerun if the wrapper header changes
    println!("cargo:rerun-if-changed={}/include/ffi/consensus_ffi.h", cpp_core_path);
    
    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header(format!("{}/include/ffi/consensus_ffi.h", cpp_core_path))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");
    
    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}