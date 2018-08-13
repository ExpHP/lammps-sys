extern crate make_cmd;
extern crate num_cpus;

extern crate bindgen;
extern crate path_abs; // better error messages
extern crate walkdir;
#[macro_use] extern crate extension_trait;

// ----------------------------------------------------

macro_rules! vec_from_features {
    ($( $feat:expr => $expr:expr, )*) => {{
        #[allow(unused_mut)]
        let mut vec = vec![];
        $( #[cfg(feature = $feat)] { vec.push($expr); })*
        vec
    }};
}

mod packages;

// ----------------------------------------------------

use ::path_abs::{PathArc, PathDir, PathFile, FileRead, FileWrite};
type BoxResult<T> = Result<T, Box<std::error::Error>>;
use ::walkdir::WalkDir;

use ::std::path::Path;
use ::std::process::{Command, Stdio};
use ::std::io::Result as IoResult;
use ::std::io::BufReader;
use ::std::fmt::{self, Display};
use ::std::io::prelude::*;
use ::std::borrow::Borrow;

// ----------------------------------------------------
// "Constants". Sorta.
// In any case, these require continued maintenence so that they
// accurately reflect the directory structure.

fn lammps_repo_dir() -> PathDir {
    const PATH: &'static str = "lammps";
    // This library might do bad things if lmp_dir is a symlink,
    // due to path canonicalization...
    let msg = "Could not find lammps submodule";
    assert!(!PathArc::new(PATH).symlink_metadata().expect(msg).file_type().is_symlink());
    PathDir::new(PATH).expect(msg)
}

// ----------------------------------------------------

fn main() {
    fn inner() -> PanicResult<()> {
        _main_print_reruns()?;

        let meta = _main_do_static_build()?;

        _main_gen_bindings(meta)?;
        Ok(())
    }
    inner().unwrap();
}

// ----------------------------------------------------

// Information discovered during the build that is
// needed during bindgen.
struct BuildMeta {
    // a bunch of "-DFLAG" args, "-D" included
    defines: CcFlags,
}

fn _main_do_static_build() -> PanicResult<BuildMeta> {
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
            "bigbig"     => "LAMMPS_BIGBIG".into(),
            // TODO: scout the LAMMPS docs/codebase for more
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
        ::make::nojay(&lmp_dir).arg(rule).run_custom().unwrap();
    }

    // Make src/STUBS/libmpi_stubs.a
    // Needed for serial builds. Quick and harmless to build for other builds.
    // Don't worry; the Makefile will determine whether we actually *use* it.
    ::make::nojay(&lmp_dir).arg("mpi-stubs").run_custom().unwrap();

    // Make src/liblammps.a
    ::make::run_fast_and_loose(
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

    Ok(BuildMeta { defines })
}

// ----------------------------------------------------

fn _main_gen_bindings(meta: BuildMeta) -> PanicResult<()> {
    let BuildMeta { defines } = meta;

    let lmp_dir = lammps_repo_dir();
    let out_path = PathDir::new(env::expect("OUT_DIR"))?;

    let _ = ::std::fs::create_dir(out_path.join("codegen"));

    // Make a bindgen builder with flags shared by both invocations.
    // (these things don't implement Clone...)
    let make_gen = || {
        let mut gen = ::bindgen::Builder::default();

        // Lammps' poorly-named header file...
        gen = gen.header(lmp_dir.join("src/library.h").display().to_string());

        // Ensure that the header contains the right features corresponding
        // to what was enabled (e.g. `LAMMPS_EXCEPTIONS`).
        gen = gen.clang_args(defines.to_args());

        // support older versions of libclang, which will mangle even
        // the names of C functions unless we disable this.
        gen = gen.trust_clang_mangling(false);
        gen
    };

    make_gen()
        .whitelist_function("lammps.*")
        .generate()
        .expect("Unable to generate bindings for 'lammps'!")
        .write_to_file(out_path.join("codegen/lammps.rs"))
        .expect("Couldn't write bindings for 'lammps'!");

    // Segregate any other bindings into a separate module.
    make_gen()
        // NOTE: Despite the name, this method also happens to
        //       blacklist functions, which is precisely what we need.
        .blacklist_type("bindings.*")
        .generate()
        .expect("Unable to generate bindings for 'other'!")
        .write_to_file(out_path.join("codegen/other.rs"))
        .expect("Couldn't write bindings for 'other'!");

    Ok(())
}

// ----------------------------------------------------

fn _main_print_reruns() -> PanicResult<()> {
    // Because we clean 'source' by deleting *literally everything*, there's no point
    // in checking it for any changes. Only the checked-out commit hash matters.
    let git_dir = lammps_dotgit_dir()?;
    assert!(git_dir.join("HEAD").exists());
    rerun_if_changed(git_dir.join("HEAD").display());

    rerun_if_changed("Cargo.toml");
    rerun_if_changed_recursive("src".as_ref())?;

    let file = BufReader::new(FileRead::read("build-data/rerun-if-env-changed")?);
    read_simple_lines(file, "#")?.into_iter().for_each(rerun_if_env_changed);
    Ok(())
}

#[allow(unused)]
fn rerun_if_changed_recursive(root: &Path) -> PanicResult<()> {
    for entry in WalkDir::new(root) {
        let entry = entry?;
        rerun_if_changed(entry.path().display());
    }
    Ok(())
}

fn rerun_if_changed<T: Display>(path: T) { println!("cargo:rerun-if-changed={}", path); }
fn rerun_if_env_changed<T: Display>(var: T) { println!("cargo:rerun-if-env-changed={}", var); }

fn lammps_dotgit_dir() -> BoxResult<PathDir> {
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

// Read lines of a simple text format where:
// - comments begin with a certain unescapable delimiter and may appear inline
// - surrounding whitespace is irrelevant
// - empty lines are skipped
// - newlines are omitted from the result
fn read_simple_lines<R: BufRead>(f: R, comment: &str) -> IoResult<Vec<String>> {
    let lines: Result<Vec<_>, _> = f.lines().collect();
    Ok(lines?.into_iter()
        .map(|s| s.split(comment).next().unwrap().trim().to_string())
        .filter(|s| s != "")
        .collect())
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

/// A result type that is always Ok because it panics otherwise.
///
/// Used whenever I'm too lazy to do any better.
pub type PanicResult<T> = Result<T, Never>;

#[derive(Debug, Clone)]
pub enum Never {}
impl<T: Display> From<T> for Never {
    fn from(e: T) -> Never { panic!("{}", e); }
}

// ----------------------------------------------------

mod env {
    #[allow(unused_imports)]
    use super::*;
    use ::std::env;

    pub fn makefile() -> PathFile {
        let var = "RUST_LAMMPS_MAKEFILE";
        match get_rerun_nonempty(var) {
            None => {
                let path = lammps_repo_dir().join("src/MAKE/Makefile.serial");
                PathFile::new(&path)
                    .unwrap_or_else(|e| panic!("Bug in lammps-sys!: {}", e))
            },
            Some(path) => {
                // user-oriented error message; mention the env var.
                PathFile::new(path)
                    .unwrap_or_else(|e| panic!("Error in {}: {}", var, e))
            },
        }
    }

    // For vars that cargo provides, like OUT_DIR.
    // This doesn't do "rerun-if-env-changed".
    pub fn expect(var: &str) -> String {
        env::var(var)
           .unwrap_or_else(|e| panic!("error reading {}: {}", var, e))
    }

    fn get_rerun_nonempty(s: &str) -> Option<String> {
        get_rerun(s).and_then(|s| match &s[..] {
            "" => None,
            _ => Some(s),
        })
    }

    fn get_rerun(s: &str) -> Option<String> {
        rerun_if_env_changed(s);
        env::var(s).map(Some).unwrap_or_else(|e| match e {
            env::VarError::NotPresent => None,
            env::VarError::NotUnicode(e) => panic!("var {} is not unicode: {:?}", s, e),
        })
    }
}

// ----------------------------------------------------

// When conveying preprocessor flags from lammps to bindgen,
// we must parse them to be able to fix relative include paths.
//
// As you may well be aware, attempting to extract a specific
// option's values from a unix-style argument list (e.g. "get all
// of the -I directories from this argument stream") is actually
// *impossible* to do correctly without knowing the complete set
// of options implemented by the program. In other words, the
// code you are about to read willfully attempts to solve an
// impossible problem. Needless to say, it makes MANY assumptions.
//
// My apologies in advance. I saw no other way.

const SHORTS_WITH_REQUIRED_ARGS: &'static [&'static str] = &[
    "-D", "-L", "-l", "-I",
];

// A flag for the C compiler (or preprocessor or linker).
pub enum CcFlag {
    // a "-DNAME" flag (or "-DNAME=VALUE", we don't care)
    Define(String),
    // an "-Ipath/to/include" flag (or "-I" "path/to/include").
    IncludeDir(PathArc),
    // an "-Lpath/to/include" flag (or "-L" "path/to/include").
    LibDir(PathArc),
    // an "-llibrary" flag
    Lib(String),
    // an unknown argument.  We will assume it is not something
    // that would prevent the next argument from being parsed as
    // an option, because an option value starting with -I/-l/-L
    // seems pretty contrived.
    Other(String),
}

impl CcFlag {
    fn map_paths<F>(self, f: F) -> Self
    where F: FnOnce(PathArc) -> PathArc,
    {
        match self {
            CcFlag::LibDir(s) => CcFlag::LibDir(f(s)),
            CcFlag::IncludeDir(s) => CcFlag::IncludeDir(f(s)),

            c@CcFlag::Define(_) => c,
            c@CcFlag::Lib(_)    => c,
            c@CcFlag::Other(_)  => c,
        }
    }
}

pub struct CcFlags(Vec<CcFlag>);

/// Wrapper with appropriate display impl for cargo:rustc-flags
struct RustLibFlags(CcFlags);
impl Display for RustLibFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // I hope you don't need spaces in your paths, because we don't quote...
        // Also, this puts a space between the option and its arguments
        // in order to cater to cargo, who will be parsing our lib args
        // for its own evil porpoises.
        write!(f, "{}", (self.0).0.iter().map(WithSpace).join(" "))
    }
}

impl CcFlag {
    fn fmt_with_space(&self, space: &str, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CcFlag::IncludeDir(ref path) => write!(f, "-I{}{}", space, path.display()),
            CcFlag::LibDir(ref path) => write!(f, "-L{}{}", space, path.display()),
            CcFlag::Lib(ref s) => write!(f, "-l{}{}", space, s),
            CcFlag::Define(ref s) => write!(f, "-D{}{}", space, s),
            CcFlag::Other(ref s) => write!(f, "{}", s),
        }
    }
}

// Displays as "-l iberty"
struct WithSpace<C>(C);
impl<C> fmt::Display for WithSpace<C> where C: Borrow<CcFlag> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    { self.0.borrow().fmt_with_space(" ", f) }
}

// Displays as "-liberty"
struct WithoutSpace<C>(C);
impl<C> fmt::Display for WithoutSpace<C> where C: Borrow<CcFlag> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    { self.0.borrow().fmt_with_space("", f) }
}

// Wrappers with appropriate display impls

impl CcFlags {
    fn parse(string: &str) -> Self {
        enum Class<'a> {
            OptWithArg(&'a str, &'a str),
            Other(&'a str),
        }

        let mut words: Vec<_> = string.split_whitespace().rev().collect();
        let mut out = vec![];
        while let Some(first) = words.pop() {

            let class = 'class: loop {
                for &prefix in SHORTS_WITH_REQUIRED_ARGS {
                    if first == prefix {
                        let panic = || panic!("{} with no argument", prefix);
                        let arg = words.pop().unwrap_or_else(panic);
                        break 'class Class::OptWithArg(prefix, arg);
                    } else if first.starts_with(prefix) {
                        break 'class Class::OptWithArg(prefix, &first[prefix.len()..]);
                    }
                }
                break 'class Class::Other(first.into());
            };

            out.push(match class {
                Class::OptWithArg("-l", s) => CcFlag::Lib(s.into()),
                Class::OptWithArg("-D", s) => CcFlag::Define(s.into()),
                Class::OptWithArg("-I", s) => CcFlag::IncludeDir(PathArc::new(s)),
                Class::OptWithArg("-L", s) => CcFlag::LibDir(PathArc::new(s)),
                Class::OptWithArg(opt, _) => panic!("Missing match arm for {}", opt),
                Class::Other(s) => CcFlag::Other(s.into()),
            })
        }
        CcFlags(out)
    }

    fn to_args(&self) -> Vec<String> {
        self.0.iter().map(|x| WithoutSpace(x).to_string()).collect()
    }

    // Canonicalize pathlike vars.
    // This is idempotent; after you do it once, all paths are absolute.
    fn make_paths_absolute(self, root: &PathDir) -> Self {
        CcFlags({
            self.0.into_iter()
                .map(|x| x.map_paths(|path| root.join(path)))
                .collect()
        })
    }
}

// ----------------------------------------------------

use makefile::LammpsMakefile;
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

//-------------------------------------------

extension_trait! {
    <I, T> IteratorJoinExt for I
    where
        I: Iterator<Item=T>,
        T: ToString
    {
        fn join(self, sep: &str) -> String
        { self.fold(String::new(), |a, b| a + sep + &b.to_string()) }
    }
}

extension_trait! {
    PathDirExt for PathDir {
        // missing functionality from path_abs
        // FIXME PanicResult due to sloth
        fn rename<Q: AsRef<Path>>(&self, dest: Q) -> BoxResult<PathDir> {
            let dest = dest.as_ref();
            ::std::fs::rename(self, dest)?;
            // Fixme
            Ok(PathDir::new(dest)?)
        }
    }
}

