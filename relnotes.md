# `lammps-sys` release notes
## v0.5.0 (Feb 27 2018)
- Add features for all packages.
- Remove default feature for `exceptions`. Default features are too hard to disable.
## v0.4.0 (Feb 27 2018)
- Automatically builds LAMMPS from source now.
- Completely different building model.  Formerly, only dynamic was supported; now, only static is supported.
  It is now possible to simply add `lammps-sys` as a dependency and have it Just Work... if you are lucky.
## v0.3.0 (Jan 20 2018)
- Use bindgen's `trust_clang_mangling(false)`.  This prevents the erroneous introduction of mangled `#[link_name]` attributes on systems with older libclang versions.
## v0.2.0
## v0.1.0

