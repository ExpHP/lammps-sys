extern crate bindgen;
use std::env;
use std::path::PathBuf;

fn main() {

    // Tell cargo to tell rustc to link the system lammps shared library.
    println!("cargo:rustc-link-lib=lammps");

    if cfg!(not(feature = "system-mpi")) {
        let path_separator = ":"; // FIXME: windows?
        let path = match env::var("C_INCLUDE_PATH") {
            Ok(p) => p,
            Err(env::VarError::NotPresent) => "".to_string(),
            Err(e) => panic!("{}", e),
        };
        let path = format!("fake-system{}{}", path_separator, path);
        env::set_var("C_INCLUDE_PATH", path);
    }

    // Generate bindings at build time.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut gen = ::bindgen::Builder::default();
    gen = gen.header("src/wrapper.h");
    gen = gen.whitelisted_function("lammps.*");

    if cfg!(not(feature = "system-mpi")) {
        gen = gen.hide_type("([oOpP])?[mM][pP][iI].*");
    }

    gen.generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
