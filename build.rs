extern crate bindgen;
use std::env;
use std::path::PathBuf;

fn main() {

    // Tell cargo to tell rustc to link the system lammps shared library.
    println!("cargo:rustc-link-lib=lammps");

    if cfg!(not(feature = "system-mpi")) {
        // Forcibly place our own mpi.h as the highest priority include path.
        let path_separator = ":"; // FIXME: windows?
        let path = match env::var("C_INCLUDE_PATH") {
            Ok(p) => p,
            Err(env::VarError::NotPresent) => "".to_string(),
            Err(e) => panic!("{}", e),
        };
        let path = format!("src/fake-system{}{}", path_separator, path);
        env::set_var("C_INCLUDE_PATH", path);
    }

    // Generate bindings at build time.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut gen = ::bindgen::Builder::default();
    gen = gen.header("src/wrapper.h");
    gen = gen.whitelist_function("lammps.*");

    // support older versions of libclang, which will mangle even
    // the names of C functions unless we disable this.
    gen = gen.trust_clang_mangling(false);

    if cfg!(not(feature = "system-mpi")) {
        gen = gen.blacklist_type("([oOpP])?[mM][pP][iI].*");
    }

    gen.generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
