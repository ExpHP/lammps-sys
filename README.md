# `lammps-sys`

[![License](https://img.shields.io/crates/l/lammps-sys.svg)](COPYING)
[![Documentation](https://docs.rs/lammps-sys/badge.svg)](https://docs.rs/lammps-sys)
[![Crates.io Version](https://img.shields.io/crates/v/lammps-sys.svg)](https://crates.io/crates/lammps-sys)
<!-- [![Build Status](https://travis-ci.org/ExpHP/lammps-sys.svg?branch=master)](https://travis-ci.org/ExpHP/lammps-sys) -->

Builds and generates Rust bindings for the C interface of LAMMPS, the [*Large-scale Atomic/Molecular Massively Parallel Simulator.*](http://lammps.sandia.gov/)

## Usage

`lammps-sys` is available on crates.io! (actually *seriously* this time).

<!-- Please remember to update ALL TOML examples, not just this one! -->
```toml
[dependencies]
lammps-sys = "0.5.2"
```

## Docs

<!-- NOTE: The cpp file has the doc comments, not the h file -->
See LAMMPS' [`library.cpp`].  This is the file that bindings will be generated to.

If you just want to see the rust signatures for the bindings, you can also generate those yourself:

```
git clone https://github.com/ExpHP/lammps-sys
cd lammps-sys
git submodule update --init
cargo doc --open
```

## Modes of operation

`lammps-sys` will first probe for a system `liblammps` using `pkg-config`, and, failing that, will build it from source. This behavior may also be configured through the `RUST_LAMMPS_SOURCE` environment variable.

See the following documents for additional information:

### Linking a system library

See [Linking a system LAMMPS library](doc/linking-a-system-library.md).

### Building from source

See [Automatically building LAMMPS from source](doc/building-from-source.md).

## Configuration

### Environment variables

The following environment variables are used by `lammps-sys` to control the build:

* **`RUST_LAMMPS_SOURCE`**
  * `RUST_LAMMPS_SOURCE=auto`:  Try to link a system library, else build from source. **(default)**
  * `RUST_LAMMPS_SOURCE=system`:  Always link the system lammps library (else report an error explaining why this failed)
  * `RUST_LAMMPS_SOURCE=build`:  Always build from source

### Cargo features

#### `exceptions`

Enables the following API functions by ensuring that `LAMMPS_EXCEPTIONS` is defined:

```
lammps_has_error
lammps_get_last_error_message
```

The system library will be skipped if it was not built with the definition.

#### Optional packages

There are a number of cargo features named with the prefix `package-`.  These are in one-to-one correspondence with LAMMPS' optional features [documented here](https://lammps.sandia.gov/doc/Packages.html).  Activating the feature `"package-user-misc"` corresponds to supplying the cmake file with `-DPKG_USER-MISC=yes`, which in turn has a similar effect to running `make yes-user-misc` if you were to use Lammps' classic make-based build system.

You should activate features for all of the packages used directly by your crate. Unfortunately, these currently only have an effect when building LAMMPS from source (see the cautionary discussion about packages in [Linking a system LAMMPS library](doc/linking-a-system-library.md)).

Be aware that these flags are almost entirely untested, and it's possible that some of them are unusable or even produce invalid cmake flags.  Please file bug reports!

Some packages such as POEMS or REAX have additional library components that must be built.  `lammps-sys` currently does not have any special handling for these, assuming that the cmake flags take care of this.  If they work for you, that's great!  If not, please file an issue.

## Does it work?

For an easier time diagnosing building/linking issues, you can clone this repo and try running the `link-test` example.

```sh
$ git clone https://github.com/ExpHP/lammps-sys
$ cd lammps-sys
$ # note: submodule update is only needed when building lammps from source
$ git submodule update --init
$ cargo run --example=link-test
LAMMPS (31 Mar 2017)
Total wall time: 0:00:00
```

Be sure to try this using the environment variables and `--features` that you plan to enable in your own project.

## License

Like Lammps, `lammps-sys` is licensed under the (full) GNU GPL v3.0. Please see the file [`COPYING`](COPYING) for more details.

## Release notes

See [Release notes](relnotes.md).

## Citations

S. Plimpton, **Fast Parallel Algorithms for Short-Range Molecular Dynamics**, J Comp Phys, 117, 1-19 (1995)

<!-- These links should all be maintained to point to the version
     of lammps that is built by `lammps-sys`                      -->
[`src/MAKE`]: https://github.com/lammps/lammps/tree/patch_5Feb2018/src/MAKE
[`library.cpp`]: https://github.com/lammps/lammps/blob/patch_5Feb2018/src/library.cpp
[the lammps source]: https://github.com/lammps/lammps/tree/patch_5Feb2018
