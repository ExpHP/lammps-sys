// omp-test-lite - The smallest possible crate using lammps-sys with omp

// Nice to use for debugging shared library linkage due to the
// tiny assembly produced.
//
// Beware! This script will almost always exit with an exit code of 0,
// even if Lammps was built without proper OpenMP support!
// You must read the STDOUT from lammps to determine if it succeeded!
//
// See the non-lite version for something that returns a better exit code.

extern crate lammps_sys;

use ::std::os::raw::{c_char, c_void, c_int};

macro_rules! stack_c_string {
    (let $name:ident : *mut c_char = $s:expr;) => {
        // copy from static memory to stack
        let mut $name = *$s;
        // get pointer to stack data
        let $name = $name.as_mut_ptr() as *mut c_char;
    }
}

fn main() {
    let mut lmp: *mut c_void = ::std::ptr::null_mut();
    unsafe {
        ::lammps_sys::lammps_open_no_mpi(
            1 as c_int,
            &mut (&mut (0 as c_char) as *mut _),
            &mut lmp,
        );

        {
            stack_c_string!{ let cmd: *mut c_char = b"package omp 0\0"; }
            ::lammps_sys::lammps_command(lmp, cmd);
        }

        // we *could* check lammps' error flag and maybe abort, but it wouldn't
        // catch any of the problems for which lammps only generates warnings.

        ::lammps_sys::lammps_close(lmp);
    }
}

