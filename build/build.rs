// automated builds of lammps from source

use ::{BoxResult, PanicResult, IoResult};
use ::{BuildMeta, CcFlag, CcFlags, RustLibFlags, WithoutSpace};
use ::{env, rerun_if_changed};
use ::std::{
    io::prelude::*,
    io::BufReader,
    process::{Command, Stdio},
};
use ::path_abs::{PathArc, PathDir, PathFile, FileWrite};
#[allow(unused)] // rust-lang/rust#45268
use ::IteratorJoinExt;

const SUBMODULE_PATH: &'static str = "lammps";

// ----------------------------------------------------

/// Build lammps from source and emit linker flags
pub(crate) fn build_from_source_and_link() -> PanicResult<BuildMeta> {
    let lmp_dir = lammps_repo_dir();
    let orig_path = env::makefile();
    rerun_if_changed(&orig_path.as_path().display());

    make::so_clean_its_like_its_not_even_there()?;

    // Create the Makefile.
    let (defines, lib_flags);
    {
        // Begin with the user specified Makefile.
        let mut makefile = LammpsMakefile::from_reader(BufReader::new(orig_path.read()?))?;

        // Append some "-D" flags to the LMP_INC line for features.
        // (even if the flags are already there, this is harmless)
        let mut defs = makefile.var_def("LMP_INC").flags();
        defs.0.append(&mut vec_from_features![
            "exceptions" => "LAMMPS_EXCEPTIONS".into(),
            // (don't bother with LAMMPS_BIGBIG for now; if the user needs that, they should
            //  customize the Makefile or use a system installation)
        ].into_iter().map(CcFlag::Define).collect());
        makefile.var_def_mut("LMP_INC").set_flags(defs.0);

        // Those are the ONLY modifications we make to the makefile.
        let makefile = makefile;

        let dir = PathDir::create_all(lmp_dir.join("src/MAKE/MINE"))?;
        let file = FileWrite::create(dir.join("Makefile.rust"))?;
        makefile.to_writer(file)?;

        //-----------------------
        // NOTE: Various flags are collected from the makefile.

        // This build script will actually parse the flags in order to fix the -I and -L
        // paths to be absolute. This allows things like "MPI_PATH = ../STUBS" in the
        // Makefile to "just work."
        let rel_to = lmp_dir.join("src/MAKE").canonicalize()?.into_dir()?;

        // These are collected for bindgen, so that it has the right preprocessor
        // definitions and can find all the necessary .h files.
        defines = makefile.gather_flags(&[
            "LMP_INC",
            "MPI_INC", "FFT_INC", "JPG_INC",
        ]).make_paths_absolute(&rel_to);

        // These are collected for rustc. This is necessary when building static libraries
        // because the final linker command will be produced by rustc.
        lib_flags = makefile.gather_flags(&[
            "MPI_PATH", "FFT_PATH", "JPG_PATH",
            "MPI_LIB",  "FFT_LIB",  "JPG_LIB",
        ]).make_paths_absolute(&rel_to);
    }; // scope

    for rule in ::packages::rules_from_features() {
        make::nojay(&lmp_dir).arg(rule).run_custom().unwrap();
    }

    // Make src/STUBS/libmpi_stubs.a
    // Needed for serial builds. Quick and harmless to build for other builds.
    // Don't worry; the Makefile will determine whether we actually *use* it.
    make::nojay(&lmp_dir).arg("mpi-stubs").run_custom().unwrap();

    // Make src/liblammps.a
    make::run_fast_and_loose(
        &lmp_dir,
        |c| c.arg("rust").arg("mode=lib"),
    ).unwrap();

    println!("cargo:rustc-link-search={}", lmp_dir.join("src").display());
    println!("cargo:rustc-link-lib=static=lammps");

    // FIXME: Does this cause problems for other crates that need libstdc++?
    //        Should there be a stdcpp-sys crate just for this?
    // NOTE: This is only needed for static builds.
    println!("cargo:rustc-flags=-l stdc++");

    // Forward the -l/-L flags to rustc, for reasons discussed above.
    println!("cargo:rustc-flags={}", RustLibFlags(lib_flags));

    Ok(BuildMeta {
        // lammps' poorly-named header file
        header: "src/library.h",
        include_dirs: CcFlags(vec![CcFlag::IncludeDir(lmp_dir.into())]),
        defines,
    })
}

// ----------------------------------------------------

/// Path to the lammps git submodule
pub(crate) fn lammps_repo_dir() -> PathDir {
    // This library might do bad things if lmp_dir is a symlink,
    // due to path canonicalization...
    let msg = "Could not find lammps submodule";
    assert!(!PathArc::new(SUBMODULE_PATH).symlink_metadata().expect(msg).file_type().is_symlink());
    PathDir::new(SUBMODULE_PATH).expect(msg)
}

/// Path to the .git directory for the lammps submodule.
pub(crate) fn lammps_dotgit_dir() -> BoxResult<PathDir> {
    // HACK: git submodules handled normally have a ".git file"
    //       containing the path to the true .git.
    //       ...but cargo does not handle submodules normally when the
    //       crate is built as an external dependency, so we must be
    //       equipped to handle both cases.
    let mut path = lammps_repo_dir().join(".git").canonicalize()?;
    while path.is_file() {
        let text = ::std::fs::read_to_string(&path)?;
        let line = text.lines().next().expect("empty .git file!");

        assert!(text.starts_with("gitdir:"));
        let line = &line["gitdir:".len()..];

        path = {
            PathArc::new(path.parent().unwrap()).join(line.trim()).canonicalize()?
        };
    }
    Ok(PathDir::new(path)?)
}

// ----------------------------------------------------

extension_trait!{
    CommandExt for Command {
        fn run_custom(&mut self) -> PanicResult<()> {
            eprintln!("Running: {:?}", self);
            // the global stdout is for cargo.
            // FIXME: what if stdout has useful info...?
            assert!(self.stdout(Stdio::null()).status()?.success());
            Ok(())
        }

        fn with_mut<F>(mut self, f: F) -> Self
        where F: FnOnce(&mut Self) -> &mut Self,
        { f(&mut self); self }
    }
}

// ----------------------------------------------------

mod make {
    use super::*;

    pub fn so_clean_its_like_its_not_even_there() -> PanicResult<()> {
        // lammps' own "make clean-all" simply does not even
        // come close to cutting it.
        let path = PathFile::new("scripts/clear-lammps")?;
        Command::new(path.as_path()).run_custom()?;
        Ok(())
    }

    pub fn jay(lmp_dir: &PathDir) -> Command {
        nojay(lmp_dir).with_mut(|c| c.arg(format!("-j{}", ::num_cpus::get() + 1)))
    }

    // For makefile rules that seem to have incomplete dependency lists.
    pub fn nojay(lmp_dir: &PathDir) -> Command {
        ::make_cmd::make()
            .with_mut(|c| c.current_dir(lmp_dir.join("src")))
    }

    // HACK
    // Runs 'make' multiple times with different settings in an attempt
    // to do as much compilation in parallel as possible, even if it
    // sporadically fails due to a poorly written Makefile.
    pub fn run_fast_and_loose<F>(lmp_dir: &PathDir, add_args: F) -> PanicResult<()>
    where F: FnMut(&mut Command) -> &mut Command,
    {
        let mut add_args = &mut { add_args };
        let mut run = move |c: Command| c.with_mut(&mut add_args).run_custom();

        // Get as much done as possible.
        // Don't keep going on failure yet; we might make better use of our cores
        // by clearing the speedbump ASAP (this theory has not been tested)
        let _ = run(jay(lmp_dir));
        // Try to get more done.
        // Get as much as possible this time as we're about to go serial.
        let _ = run(jay(lmp_dir).with_mut(|c| c.arg("--keep-going")));
        // Okay. Get the rest in serial.
        run(nojay(lmp_dir))
    }
}


// ----------------------------------------------------

use self::makefile::LammpsMakefile;
mod makefile {
    use super::*;
    pub struct LammpsMakefile(Vec<String>);

    impl LammpsMakefile {
        pub fn from_reader<R: BufRead>(r: R) -> IoResult<Self> {
            Ok(LammpsMakefile(r.lines().collect::<Result<_,_>>()?))
        }

        pub fn to_writer<W: Write>(&self, mut w: W) -> IoResult<()> {
            for line in &self.0 {
                writeln!(w, "{}", line)?;
            }
            Ok(())
        }

        // get a handle for reading a simple variable assignment
        pub fn var_def(&self, name: &str) -> VarDef {
            let (line, start_col) = self._expect_var_def_data(name);
            VarDef_ { makefile: self, line, start_col }
        }

        pub fn var_def_mut(&mut self, name: &str) -> VarDefMut {
            let (line, start_col) = self._expect_var_def_data(name);
            VarDef_ { makefile: self, line, start_col }
        }

        pub fn gather_flags(&self, vars: &[&str]) -> CcFlags {
            let strings = vars.iter().map(|v| self.var_def(v).text().to_string());
            CcFlags::parse(&strings.join(" "))
        }

        fn _expect_var_def_data(&self, name: &str) -> (usize, usize) {
            self._var_def_data(name)
                .unwrap_or_else(|| {
                    panic!("could not locate {} definition in makefile", name)
                })
        }

        fn _var_def_data(&self, name: &str) -> Option<(usize, usize)> {
            // FIXME this parser is very dumb and incorrect.

            let is_identifier_char = |c| match c {
                b'a'...b'z' | b'A'...b'Z' | b'_' => true,
                _ => false,
            };

            // Make sure there's only one relevant line and that it is
            // a simple '=' declaration; no tricks.
            let mut matches = self.0.iter().enumerate()
                // (don't know if variable defs can be indented; don't care)
                .filter(|&(_, line)|
                    line.starts_with(name)
                        && line.len() > name.len()
                        && !is_identifier_char(line.as_bytes()[name.len()])
                );
            let (index, line) = matches.next()?;
            assert!(matches.next().is_none(), "Too many '{}' lines!", name);

            let eq_index = line.find("=")?;

            // e.g. no '+='
            assert_eq!(line[..eq_index].trim(), name, "Strange '{}' line!", name);

            assert!(!line.ends_with("\\"), "continued lines not supported");

            Some((index, eq_index + 1))
        }
    }

    // Simple abstraction for reading and writing the RHS of a
    // simple variable assignment in a makefile.
    pub type VarDef<'a> = VarDef_<&'a LammpsMakefile>;
    pub type VarDefMut<'a> = VarDef_<&'a mut LammpsMakefile>;

    pub struct VarDef_<T> {
        makefile: T,
        line: usize,
        start_col: usize,
    }

    impl<T: ::std::ops::Deref<Target=LammpsMakefile>> VarDef_<T> {
        /// Read the variable's definition
        pub fn text(&self) -> &str {
            let VarDef_ { ref makefile, line, start_col } = *self;
            &(**makefile).0[line][start_col..]
        }

        pub fn flags(&self) -> CcFlags
        { CcFlags::parse(self.text()) }
    }

    impl<T: ::std::ops::DerefMut<Target=LammpsMakefile>> VarDef_<T> {
        /// Write a value for the variable
        pub fn set_text<S: AsRef<str>>(&mut self, s: S) {
            let s = s.as_ref();

            assert!(!s.ends_with("\\")); // what are you trying to pull?

            let VarDef_ { ref mut makefile, line, start_col } = *self;
            let line = &mut (**makefile).0[line];
            line.truncate(start_col);
            *line += s;
        }

        pub fn set_flags<Ss>(&mut self, iter: Ss)
        where Ss: IntoIterator<Item=CcFlag>,
        { self.set_text(iter.into_iter().map(WithoutSpace).join(" ")) }
    }
}
