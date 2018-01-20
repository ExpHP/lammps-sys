// link_test - The smallest possible crate using lammps_sys
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
