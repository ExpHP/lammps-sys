# Linking a system Lammps library

`lammps-sys` will automatically probe for a system `liblammps`, and use it if found.

The search for a system library can be disabled by setting `RUST_LAMMPS_SOURCE=build`, and the fallback of building from source can be disabled by setting `RUST_LAMMPS_SOURCE=system`.

## Linking Lammps itself

`lammps-sys` uses `pkg-config` to locate the LAMMPS C library.  The traditional way of building lammps does not install the library in any manner (or produce the requisite pkgconfig file), but you can set this up yourself.  Lammps also comes with cmake files, which will produce an *almost* working setup once installed.

### Tips to building

If you want cmake to install headers and the pkgconfig file, you'll need to supply `-DBUILD_LIB=yes -DBUILD_SHARED_LIBS=yes` to the initial `cmake` command. (notice that it does not install headers or pkgconfig info when building a static library).

If you're going the cmake route, you are advised not to use `stable_22Aug2018` release.  It has numerous bugs in its CMakeLists.txt that are fixed in the following patch releases, such as a dysfunctional `PKG_USER-OMP`, and a trailing `@` in the `liblammps.pc` file.  The first "good" release is `patch_18Sep2018`.

### Example installation

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

**`liblammps.pc`**
```
prefix=/home/lampam/.local
libdir=${prefix}/lib
includedir=${prefix}/include

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

To enable MPI, "simply" enable the `"mpi"` cargo feature.  When enabled, `lammps-sys` exposes additional functions whose signatures involve MPI types; these will be assigned types from the `mpi-sys` crate, for compatibility with the `mpi` crate.

The library must have been built against the same implementation of MPI that is currently associated with the `mpicc` compiler wrapper.  Otherwise, you will have a not-so-fun time (read: segfaults).  For a small amount of increased confidence, try building the MPI link test:

```
cargo run --example=mpi-test --features=mpi
```

## Dealing with missing features

There is, of course, the issue that the system lammps library may have been built without certain features that your application requires.

`lammps-sys` will perform a small number of sanity checks on the system library before deciding to use it (such as making sure `-DLAMMPS_EXCEPTIONS` was supplied if you activate the `exceptions` feature).  However, these checks are far from comprehensive.

**`lammps-sys` does not currently verify that the system `liblammps` includes optional packages like `MANYBODY`.**  Even if you activate the corresponding cargo features, it will happily link a library that is missing these packages, and this error will go entirely unnoticed until the program fails at runtime when it tries to use the package.  This papercut may be fixed in the future.

For now, if the situation arises that there is a system lammps library which you cannot or do not wish to use, it is recommended that you set `RUST_LAMMPS_SOURCE=build` in your environment to disable the system library search.
