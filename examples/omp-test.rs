// omp-test - Tests lammps with 'package omp' and panics if things don't look right.

// Usage:
//
//     cargo run --example=omp-test --features=user-omp  [OTHER_CARGO_ARGS]...
//
// If this is your first time trying it, then *almost certainly* this will fail.
// Some external setup IS required to make OpenMP work in `lammps-sys`.
// Please see the top-level README.md for more information.
//
// If OpenMP is set up properly, you will see LAMMPS create OMP_NUM_THREADS threads:
// (e.g. in this output OMP_NUM_THREADS=1)
//
//     LAMMPS (5 Feb 2018)
//       using 1 OpenMP thread(s) per MPI task
//     using multi-threaded neighbor list subroutines
//     Total wall time: 0:00:00
//
// If it doesn't look like OpenMP is set up properly, this crate will panic.

extern crate lammps_sys;

use ::std::os::raw::{c_char, c_void, c_int};
use ::std::io::{BufRead, BufReader};

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

        #[cfg(feature = "exceptions")] {
            if ::lammps_sys::lammps_has_error(lmp) != 0 {
                panic!("Aborting due to above error from lammps!");
            }
        }

        // Close lammps now so the logfile gets flushed.
        ::lammps_sys::lammps_close(lmp);

        {
            // There are some conditions in which LAMMPs will only print a warning
            // and continue with one thread. For instance, it will do this if
            // "-fopenmp" was not supplied during compilation of the *.o files.
            //
            // Therefore, we search the logfile for warnings.
            let file = ::std::fs::File::open("log.lammps").expect("could not read log.lammps");
            for line in BufReader::new(file).lines() {
                let line = line.expect("Error reading line from log.lammps");
                if line.starts_with("WARNING") {
                    panic!("Aborting due to above warning from lammps!")
                }
            }
        }
    }
}

