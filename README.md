# `lammps-sys`

Automatically-generated Rust bindings for the C interface of LAMMPS, the [*Large-scale Atomic/Molecular Massively Parallel Simulator.*](http://lammps.sandia.gov/)

## Usage

`lammps-sys` is not currently on crates.io.  You can depend on it with a git dependency:

```toml
[dependencies.lammps-sys]
tag = "v0.2.0"
git = "https://github.com/ExpHP/lammps-sys"
```

## Some assembly required

### Building lammps

You will likely need to build LAMMPS manually in order to enable some non-standard options:

* It must be built as a **shared library.**
* It must be built with `-DLAMMPS_EXCEPTIONS`.

An example of how you can achieve this:

```sh
cd where/you/unpacked/lammps
cd src

# Create a custom makefile.
# You can base it off of any file in MAKE, this just uses 'omp' as an example
cp MAKE/OPTIONS/Makefile.omp MAKE/MINE/Makefile.my-omp
nano MAKE/MINE/Makefile.my-omp # find LMP_INC and add -DLAMMPS_EXCEPTIONS
                               # to the end of the line

make my-omp mode=lib
```

### Installing LAMMPS where `lammps-sys` can find it

Building lammps will produce a `liblammps_*.so` file in `src`.  Also in in the lammps `src` directory is a file named `library.h`.

1. Install `liblammps_*.so` somewhere in `LD_LIBRARY_PATH` under the name **`liblammps.so`**

2. Install `library.h` somewhere in `C_INCLUDE_PATH` as **`lammps/library.h`**.

### MPI

By default, **`MPI_Comm`** is defined as an empty type, forbidding usage of the `lammps_open` function. To instantiate LAMMPS under the default settings, **you must use `lammps_open_no_mpi`**.

However, *if you must:*

```toml
[dependencies.lammps-sys]
tag = "v0.2.0"
features = ["system-mpi"]
```
When you enable the feature **`system-mpi`**, then bindgen will search for `mpi.h` on the system path. This must correspond to **the same implementation of MPI that Lammps was built against** if you plan to call `lammps_open`. This usage of `lammps-sys` is currently unsupported, because I do not need it and it seems like a major footgun.  If you use it, [let me know how it works out.](https://github.com/ExpHP/lammps-sys/issues)

## License

Like Lammps, `lammps-sys` is licensed under the (full) GNU GPL v3.0. Please see the file `COPYING` for more details.

## Citations

S. Plimpton, **Fast Parallel Algorithms for Short-Range Molecular Dynamics**, J Comp Phys, 117, 1-19 (1995)
