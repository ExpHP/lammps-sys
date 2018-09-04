// mpi-test - The second smallest possible crate using lammps-sys

// Usage:
//
//     cargo run --example=mpi-test --features=mpi [other cargo arguments...]
//
// Successful output looks like e.g.:
//
//        Compiling lammps-sys v0.5.0 (file:///home/exp/dev/rust/lammps-sys)
//         Finished dev [unoptimized + debuginfo] target(s) in 63.48 secs
//          Running `target/debug/examples/mpi-test`
//     LAMMPS (22 Aug 2018)
//       using 4 OpenMP thread(s) per MPI task
//     Total wall time: 0:00:00
//
// The vast majority of possible problems will manifest during the linking
// of the final binary, before it is run.  I can't tell you what you'll see,
// exactly, but most likely cargo will report failure running a "cc" command,
// and exit with a nonzero status.
//
// It is also possible that the call to `lammps_open` will segfault in an
// early call to an MPI function; this would likely indicate that lammps
// and mpi_sys are linking to different implementations of MPI.

extern crate lammps_sys;
extern crate mpi_sys;

use ::std::os::raw::{c_char, c_void, c_int};

fn main() {
    let mut lmp: *mut c_void = ::std::ptr::null_mut();
    unsafe {
        ::mpi_sys::MPI_Init(
            &mut (1 as c_int),
            &mut (&mut (&mut (0 as c_char) as *mut _) as *mut _),
        );
        ::lammps_sys::lammps_open(
            1 as c_int,
            &mut (&mut (0 as c_char) as *mut _),
            mpi_sys::RSMPI_COMM_WORLD,
            &mut lmp,
        );
        let mut lol = [b'p' as i8, 'a' as i8, 'c' as i8, 'k' as i8, 'a' as i8, 'g' as i8, 'e' as i8, ' ' as i8, 'o' as i8, 'm' as i8, 'p' as i8, ' ' as i8, '0' as i8, 0 as c_char];
        ::lammps_sys::lammps_command(
            lmp,
            lol.as_mut_ptr(),
        );
        ::lammps_sys::lammps_close(lmp);
        ::mpi_sys::MPI_Finalize();
    }
}
