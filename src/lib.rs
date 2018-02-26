#![allow(bad_style)]

//! Automatically-generated bindings for lammps, using bindgen.

include!(concat!(env!("OUT_DIR"), "/codegen/lammps.rs"));

pub mod other {
    //! Bindings to other things compiled with lammps, filtered into
    //! a separate module to help reduce clutter.
    //!
    //! These can be particularly important to have on hand in case
    //! lammps was built statically.
    include!(concat!(env!("OUT_DIR"), "/codegen/other.rs"));
}