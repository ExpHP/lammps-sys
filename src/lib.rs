#![allow(bad_style)]

//! Automatically-generated bindings for lammps, using bindgen.

#[cfg(feature = "mpi")]
extern crate mpi_sys;

#[cfg(feature = "mpi")]
extern "C" {
    pub fn lammps_open(
        argc: std::os::raw::c_int,
        argv: *mut *mut ::std::os::raw::c_char,
        communicator: mpi_sys::MPI_Comm,
        lmp: *mut *mut ::std::os::raw::c_void,
    );
}

include!(concat!(env!("OUT_DIR"), "/codegen/lammps.rs"));
