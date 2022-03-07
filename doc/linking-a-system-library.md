# Linking a system Lammps library

`lammps-sys` will automatically probe for a system `liblammps`, and use it if found.

The search for a system library can be disabled by setting `RUST_LAMMPS_SOURCE=build`, and the fallback of building from source can be disabled by setting `RUST_LAMMPS_SOURCE=system`.

## How `lammps-sys` locates LAMMPS

`lammps-sys` uses `pkg-config` to locate LAMMPS.  Try running the following command to see what `lammps-sys` sees:

```
$ pkg-config --cflags --libs liblammps
-DLAMMPS_SMALLBIG -DLAMMPS_EXCEPTIONS -I/home/lampam/data/opt/lammps/include -L/home/lampam/data/opt/lammps/lib -llammps
```

Generally speaking, this means that:

* An appropriate `.pc` file must be installed.  (see the next section)
* `PKG_CONFIG_PATH` must be set to locate the lib at build time.
* `LD_LIBRARY_PATH` must be set to locate the lib at runtime, if it was built as a shared library.

## Tips to building and installing LAMMPS

* **Use [the `cmake` system](https://docs.lammps.org/Build_cmake.html) to build LAMMPS! Do not use the legacy in-tree Makefile system.**
* Check in advance which features you need for the rust code you are building before calling cmake.  For instance, [rsp2](https://github.com/ExpHP/rsp2) requires `-DLAMMPS_EXCEPTIONS=yes -DPKG_MANYBODY=yes -DPKG_USER-MISC=yes` and possibly `-DPKG_USER-OMP=yes`.
    * Older versions of the LAMMPS source tree may additionally require `-DBUILD_LIB=yes`.
* Build a shared library (`-DBUILD_SHARED_LIBS=yes`).
    * If you build a static library then LAMMPS' cmake configuration doesn't install the .pc file or headers and you will have to take care of these manually.

## Example build

Here is an example of how to build LAMMPS 17Feb2022, install it to `$HOME/opt/lammps`, and link to it from lammps-sys.

```sh
LAMMPS=$HOME/opt/lammps

# Build and install lammps
git clone https://github.com/lammps/lammps
(
    cd lammps
    git checkout patch_17Feb2022
    mkdir build
    cd build
    cmake -DLAMMPS_EXCEPTIONS=yes -DPKG_MANYBODY=yes -DPKG_USER-MISC=yes -DCMAKE_INSTALL_PREFIX=$LAMMPS -DBUILD_SHARED_LIBS=yes ../cmake
    make -j32
    make install
)

# Set environment
export PKG_CONFIG_PATH=$LAMMPS/lib/pkgconfig:$PKG_CONFIG_PATH
export LD_LIBRARY_PATH=$LAMMPS/lib:$LD_LIBRARY_PATH

# Run the example link test
git clone https://github.com/ExpHP/lammps-sys
(
    cd lammps-sys
    cargo run --example=link-test
)
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
