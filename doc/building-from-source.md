# Automatically building LAMMPS from source

Under the default settings, if a system library cannot be found, `lammps-sys` will automatically build LAMMPS from source.  The default settings hopefully work out-of-the-box on most systems, though it might not be super fast.  You can configure the build if necessary to enable features like OpenMP.
Be sure to try this using the environment variables and `--features` that you plan to enable in your own project.

## How-to

### Enabling OpenMP

Enabling OpenMP will require you to supply your own Makefile.

* If you are not familiar with the process of building LAMMPS, clone [the lammps source] and follow their instructions to learn how to compile an executable.
* Compile a LAMMPS executable with OpenMP support. You won't be *using* any of these build artefacts, but bear with me; the mere act of compiling will require you to browse around their prepackaged makefiles and (very likely) make one of your own.
  * `OPTIONS/Makefile.omp` is a good starting point.  Because MPI support in `lammps-sys` is hazy, I suggest borrowing the `MPI_` lines from `Makefile.serial` to link in the STUBS library.
  * You will know you have succeeded when the lammps binary accepts the command `"package omp 0"` and does not emit any warnings or errors.
* Save your working makefile somewhere special, and set an absolute path to it in the environment variable `RUST_LAMMPS_MAKEFILE`.

#### Supplying `-fopenmp` at linking

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

I... haven't tried it.  You can try using a custom Makefile (see [Enabling OpenMP](#enabling-openmp)).  And if everything seems to work all the way up until the linking of the final binary, you might be able to get away with a workaround like the `.cargo/config` trick to supply missing linker arguments.  In any case, [let me know how it works out for you.](https://github.com/ExpHP/lammps-sys/issues)

## Configuration

There are numerous cargo features which tweak the build.  See the toplevel [README](../README.md).
