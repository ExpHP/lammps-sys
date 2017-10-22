#![allow(bad_style)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Dummy uninstantiable definition of `MPI_Comm`.
///
/// By default, `lammps-sys` uses this definition so that you do not require
/// a definition of "mpi.h" on your system path.  To disable this behavior,
/// enable the "system-api" feature.
#[cfg(not(feature = "system-mpi"))]
pub enum MPI_Comm { }
