# Automatically building LAMMPS from source

Under the default settings, if a system library cannot be found, `lammps-sys` will automatically build LAMMPS from source.  This makes use of the CMake configuration files which were only recently added to the lammps source tree.  You can configure the build if necessary to enable features like OpenMP.

## How-to

### Enabling OpenMP

Enabling `package-user-omp` should be enough to ensure that LAMMPS gets built with OpenMP... ideally.  In the version of LAMMPS built by `lammps-sys`, there is currently [a compatibility issue with GCC >= 9.0](https://github.com/lammps/lammps/issues/1482) that will cause LAMMPS to automatically disable OpenMP if you use this compiler. (there is no workaround at present; use another compiler!)

In any case, the greater trouble is what happens on the rust side of things, *after* LAMMPS is built.

If you get errors during the linking stage about undefined symbols from `omp_`, you may try adding the following to your `~/.cargo/config` (or to a `.cargo/config` in your own crate's directory):

```
# For building lammps-sys with openmp.
# Not sure how this impacts other crates...

[target.x86_64-unknown-linux-gnu]
rustflags = ["-Clink-args=-fopenmp"]
```

(you can obtain the correct target triple for your machine by running `rustc --version -v`)

If this sounds like terrible advice, that's because it probably is!  Unfortunately, it does not seem to be possible to set this flag from within a cargo build script, and I do not know of a better solution at this time.

### Enabling MPI

You can enable the `mpi` feature to build lammps with MPI.  For this to work well, `mpicc` and `mpicxx` should be associated with the same MPI implementation. (these wrappers are used by the `mpi-sys` crate and LAMMPS' cmake file, respectively)

As with anything else, if you have trouble, [please file an issue](https://github.com/ExpHP/lammps-sys/issues)!

## Configuration

There are numerous cargo features which tweak the build.  See the toplevel [README](../README.md).
