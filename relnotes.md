# `lammps-sys` release notes
## v0.6.0 (Aug 28 2019)
- Update to `stable_7Aug2019`, to "fix" builds for GCC 9.0. (unfortunately this disables OpenMP for that compiler).  The major version has been bumped in case any backwards incompatible changes occurred in LAMMPS.
## v0.5.2 (May 10 2019)
- Fix automatic builds to actually work when using the crate files published to crates.io. (oops!)
## v0.5.1 (April 9 2019)
- Fix readme display on crates.io
## v0.5.0 (Nov 21 2018)
- Update automatically-built LAMMPS version to `patch_18Sep2018`.
- Added back the ability to use prebuilt libs.  This is automatically supported through `pkg-config`, though you probably need to set up a `liblammps.pc` file (see the files in `doc/` for assistance).
- `lammps-sys` now internally uses the CMake build system recently added to LAMMPS, rather than the classic Makefile build system.
- Removed the `RUST_LAMMPS_MAKEFILE` environment variable, which is no longer relevant with the new CMake-based builds.
- Added back MPI support.  There is now an `"mpi"` feature which enables the binding for `lammps_open`, and links to the `mpi-sys` crate. This is a great deal more reliable than v0.3.0's `"system-mpi"` feature, so don't be afraid to use it!
- Added a feature for every package. These are almost entirely untested; please report bugs!
- Remove default feature for `"exceptions"`. Default features are too hard to disable.
- Removed the `"bigbig"` feature, which did not make sense as a feature.  If you need it, build lammps as a shared library (and make sure `-DLAMMPS_BIGBIG` is present in the Makefile's `LMP_INC` and in the `cflags:` line of `liblammps.pc`)
## v0.4.0 (Feb 27 2018)
- Automatically builds LAMMPS from source now.
- Completely different building model.  Formerly, only dynamic was supported; now, only static is supported.
  It is now possible to simply add `lammps-sys` as a dependency and have it Just Work... if you are lucky.
## v0.3.0 (Jan 20 2018)
- Use bindgen's `trust_clang_mangling(false)`.  This prevents the erroneous introduction of mangled `#[link_name]` attributes on systems with older libclang versions.
## v0.2.0
## v0.1.0

