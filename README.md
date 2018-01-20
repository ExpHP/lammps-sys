# `lammps-sys`

Automatically-generated Rust bindings for the C interface of LAMMPS, the [*Large-scale Atomic/Molecular Massively Parallel Simulator.*](http://lammps.sandia.gov/)

## Usage

`lammps-sys` is not currently on crates.io.  You can depend on it with a git dependency:

```toml
[dependencies.lammps-sys]
tag = "v0.3.0"
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

make my-omp mode=shlib
```

### Installing LAMMPS where `lammps-sys` can find it

Building lammps will produce a `liblammps_*.so` file in `src`.  Also in in the lammps `src` directory is a file named `library.h`.

1. Install `liblammps_*.so` somewhere in `LIBRARY_PATH` and `LD_LIBRARY_PATH` under the name **`liblammps.so`**

2. Install `library.h` somewhere in `C_INCLUDE_PATH` as **`lammps/library.h`**.

### Docs

I recommend you look at that `library.h` file you just installed.

If you just want to see the rust signatures for the bindings, you can also generate those yourself:

```
git clone https://github.com/ExpHP/lammps-sys
cd lammps-sys
cargo doc
chromium target/doc/lammps_sys/index.html
```

### MPI

By default, **`MPI_Comm`** is defined as an empty type, forbidding usage of the `lammps_open` function. To instantiate LAMMPS under the default settings, **you must use `lammps_open_no_mpi`**.

However, *if you must:*

```toml
[dependencies.lammps-sys]
tag = "v0.2.0"
git = "https://github.com/ExpHP/lammps-sys"
features = ["system-mpi"]
```
When you enable the feature **`system-mpi`**, then bindgen will search for `mpi.h` on the system path. This must correspond to **the same implementation of MPI that Lammps was built against** if you plan to call `lammps_open`. This usage of `lammps-sys` is currently unsupported, because I do not need it and it seems like a major footgun.  If you use it, [let me know how it works out.](https://github.com/ExpHP/lammps-sys/issues)

### Did it work?

You can test your lammps install and system configuration by cloning this repo and running the `link-test` example.

```sh
$ git clone https://github.com/ExpHP/lammps-sys
$ cd lammps-sys
$ cargo run --example=link-test
LAMMPS (31 Mar 2017)
Total wall time: 0:00:00
```

## [License](COPYING)

Like Lammps, `lammps-sys` is licensed under the (full) GNU GPL v3.0. Please see the file `COPYING` for more details.

## [Release notes](relnotes.md)

## Citations

S. Plimpton, **Fast Parallel Algorithms for Short-Range Molecular Dynamics**, J Comp Phys, 117, 1-19 (1995)
