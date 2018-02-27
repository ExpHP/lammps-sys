// link-test - The smallest possible crate using lammps-sys
//
// If you were to link lammps as a shared library,
// the generated assembly would fit on a postcard.

// Usage:
//
//     cargo run --example=link-test  [other cargo arguments...]
//
// Successful output looks like e.g.:
//
//        Compiling lammps-sys v0.4.0 (file:///home/exp/dev/rust/lammps-sys)
//         Finished dev [unoptimized + debuginfo] target(s) in 63.48 secs
//          Running `target/debug/examples/link-test`
//     LAMMPS (5 Feb 2018)
//     Total wall time: 0:00:00
//
// The vast majority of possible problems will manifest during the linking
// of the final binary, before it is run.  I can't tell you what you'll see,
// exactly, but most likely cargo will report failure running a "cc" command,
// and exit with a nonzero status.

extern crate lammps_sys;

use ::std::os::raw::{c_char, c_void, c_int};

fn main() {
    let mut lmp: *mut c_void = ::std::ptr::null_mut();
    unsafe {
        ::lammps_sys::lammps_open_no_mpi(
            1 as c_int,
            &mut (&mut (0 as c_char) as *mut _),
            &mut lmp,
        );
        ::lammps_sys::lammps_close(lmp);
    }
}
