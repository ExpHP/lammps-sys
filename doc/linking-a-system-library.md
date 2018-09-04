# Linking a system Lammps library

`lammps-sys` will automatically probe for a system `liblammps`, and use it if found.

The search for a system library can be disabled by setting `RUST_LAMMPS_SOURCE=build`, and the fallback of building from source can be disabled by setting `RUST_LAMMPS_SOURCE=system`.

## Linking Lammps itself

`lammps-sys` uses `pkg-config` to locate the LAMMPS C library.  The traditional way of building lammps does not install the library in any manner (or produce the requisite pkgconfig file), but you can set this up yourself.  Lammps also comes with cmake files, which I believe will produce a working setup once installed. (**TODO:** Test this!)

Example of a suitable installation: (assuming an installation path of `$HOME/.local`)

```
/home/lampam/.local
├── include
│   └── lammps
│       └── library.h
└── lib
    ├── liblammps.so
    └── pkgconfig
        └── liblammps.pc
```

```pkgconfig
prefix=/home/lampam/.local
libdir=$prefix/lib
includedir=$prefix/include

Name: liblammps
Description: Large-scale Atomic/Molecular Massively Parallel Simulator Library
URL: http://lammps.sandia.gov
Version:
Requires:
Libs: -L${libdir} -llammps
Libs.private: -lm

# The following flags should be present if and only if lammps was built with them:
# - `-DLAMMPS_EXCEPTIONS`
# - `-DLAMMPS_BIGBIG`
Cflags: -I${includedir} -DLAMMPS_EXCEPTIONS
```

To test it:

```sh
$ export PKG_CONFIG_PATH=/home/lampam/.local/lib/pkgconfig:$PKG_CONFIG_PATH
$ pkg-config --libs --cflags liblammps
-DLAMMPS_EXCEPTIONS -llammps
$
```

## Linking MPI

`lammps-sys` currently expects MPI to be available in the system paths, which is perhaps slightly unconventional:

* `mpi.h` must be somewhere in `C_INCLUDE_PATH`.
* `libmpi.so` must be somewhere in `LIBRARY_PATH`.

The type of `MPI_Comm` will be included in the bindings, but nothing else will be; you should probably separately depend on a crate like `mpi-sys` for a complete set of MPI bindings.

Unfortunately, in some MPI implementations such as OpenMPI (which defines `MPI_Comm` as `*ompi_communicator_t`), the `MPI_Comm` from `lammps-sys` and `mpi-sys` will be considered distinct types.  A future version of `lammps-sys` may resolve this by direct integration with the `mpi-sys` crate... but for now, you will likely need to use `mem::transmute` to unsafely cast between the two `MPI_Comm` types.

### `mpi_stubs`

**Linking a system Lammps library that uses `mpi_stubs` has not been tested.**  However, it can probably be done by adding `-lmpi_stubs` to the pkgconfig file, and putting `libmpi_stubs.so` in the installation `lib` dir.  If you need help or have more information, [please open an issue](https://github.com/ExpHP/lammps-sys/issues).

## Dealing with missing features

There is, of course, the issue that the system lammps library may have been built without certain features that your application requires.

`lammps-sys` will perform a small number of sanity checks on the system library before deciding to use it (such as making sure `-DLAMMPS_EXCEPTIONS` was supplied if you activate the `exceptions` feature).  However, these checks are far from comprehensive.

Unfortunately, **there is currently no way for `lammps-sys` to verify that the system `liblammps` includes optional packages like `MANYBODY`.**  Even if you activate the corresponding cargo features, it will happily link a library that is missing these packages, and this error will go entirely unnoticed until the program fails at runtime when it tries to use fixes or potentials from the package.

Similarly, you will very likely have a not-so-fun time (read: segmentation faults) if you try to enable the `"mpi"` cargo feature when linking against a system lammps library that was not built against the same implementation reported by `mpicc --show`.

For now, if the situation arises that there is a system lammps library which you cannot or do not wish to use, it is recommended that you set `RUST_LAMMPS_SOURCE=build` in your environment to disable the system library search.
