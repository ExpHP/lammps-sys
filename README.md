# `lammps-sys`

Automatically-generated Rust bindings for the C interface of LAMMPS, the [*Large-scale Atomic/Molecular Massively Parallel Simulator.*](http://lammps.sandia.gov/)

## Some assembly required

1. This currently only supports linking LAMMPS as a **shared library.**
   - Note this is *not the default build mode of LAMMPS.*
   - Make sure that **`liblammps.so`** is available in your `LD_LIBRARY_PATH`.
2. This requires the LAMMPS C interface header file.
   - This file is located at `src/library.h` in the LAMMPS source tree.
   - You must make this available as **`lammps/library.h`** somewhere in your `C_INCLUDE_PATH`.

### MPI

By default, **`MPI_Comm`** is defined as an empty type, forbidding usage of the `lammps_open` function. To instantiate LAMMPS under the default settings, **you must use `lammps_open_no_mpi`**.

However, *if you must:*

```toml
[dependencies.lammps-sys]
version = "0.1"
features = ["system-mpi"]
```
When you enable the feature **`system-mpi`**, then bindgen will search for `mpi.h` on the system path. This must correspond to **the same implementation of MPI that Lammps was built against** if you plan to call `lammps_open`. This usage of `lammps-sys` is currently unsupported, because I do not need it and it seems like a major footgun.  If you use it, [let me know how it works out.](https://github.com/ExpHP/lammps-sys/issues)

## License

Like Lammps, `lammps-sys` is licensed under the (full) GNU GPL v3.0. Please see the file `COPYING` for more details.

## Citations

S. Plimpton, **Fast Parallel Algorithms for Short-Range Molecular Dynamics**, J Comp Phys, 117, 1-19 (1995)
