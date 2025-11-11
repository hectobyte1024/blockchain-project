use std::env;
use std::path::PathBuf;

fn main() {
    // Build the C++ core library first
    let cpp_core_path = PathBuf::from("../../cpp-core");
    
    // Use cmake to build the C++ library
    let dst = cmake::Config::new(&cpp_core_path)
        .define("BUILD_SHARED_LIBS", "OFF") // Static library for embedding
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("BUILD_TESTS", "OFF")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=blockchain_core");
    println!("cargo:rustc-link-lib=static=blockchain_ffi");
    
    // Link required system libraries
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=secp256k1");
    println!("cargo:rustc-link-lib=leveldb");
    
    // Platform-specific linking
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
        println!("cargo:rustc-link-lib=m");
    }
    
    // Generate Rust bindings from C header
    let header_path = cpp_core_path.join("include/ffi/blockchain_ffi.h");
    let consensus_header_path = cpp_core_path.join("include/ffi/consensus_ffi.h");
    
    let bindings = bindgen::Builder::default()
        .header(header_path.to_str().unwrap())
        .header(consensus_header_path.to_str().unwrap())
        .clang_arg(format!("-I{}", cpp_core_path.join("include").display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("crypto_.*")
        .allowlist_function("consensus_.*") 
        .allowlist_function("storage_.*")
        .allowlist_function("vm_.*")
        .allowlist_function("serialize_.*")
        .allowlist_function("deserialize_.*")
        .allowlist_function("hex_.*")
        .allowlist_function("byte_buffer_.*")
        .allowlist_function("c_mine_block")
        .allowlist_function("c_verify_proof_of_work")
        .allowlist_function("c_calculate_next_difficulty")
        .allowlist_function("c_should_adjust_difficulty")
        .allowlist_type("BlockchainResult")
        .allowlist_type("Hash256")
        .allowlist_type("Hash160")
        .allowlist_type("PrivateKey")
        .allowlist_type("PublicKey")
        .allowlist_type("Signature")
        .allowlist_type("Transaction.*")
        .allowlist_type("Block.*")
        .allowlist_type("OutPoint")
        .allowlist_type("ByteBuffer")
        .allowlist_type("CMiningResult")
        .allowlist_var("BLOCKCHAIN_.*")
        .derive_default(false)  // Manually implement Default to avoid conflicts
        .derive_debug(false) // Avoid conflicts with manual implementations
        .derive_copy(false)  // ByteBuffer has Drop, can't be Copy
        .derive_eq(false)
        .derive_partialeq(false)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
    
    // Tell cargo to invalidate the built crate whenever these files change
    println!("cargo:rerun-if-changed={}", header_path.display());
    println!("cargo:rerun-if-changed=../../cpp-core");
    
    // Generate C++ bindings for Rust structures (reverse FFI)
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // Temporarily disable cbindgen due to syntax parsing issues
    // We'll add this back once the core FFI is stable
    /*
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_include_guard("BLOCKCHAIN_RUST_FFI_H")
        .generate()
        .expect("Unable to generate C bindings")
        .write_to_file("../../shared/bindings/rust_ffi.h");
    */
}