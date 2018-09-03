extern crate bindgen;
extern crate path_abs; // better error messages
extern crate walkdir;
extern crate pkg_config;
extern crate cmake;
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
mod probe;
mod build;

// ----------------------------------------------------

use ::path_abs::{PathArc, PathDir, FileRead};
type BoxResult<T> = Result<T, Box<std::error::Error>>;
use ::walkdir::WalkDir;

use ::std::path::Path;
use ::std::io::Result as IoResult;
use ::std::io::BufReader;
use ::std::fmt::{self, Display};
use ::std::io::prelude::*;
use ::std::borrow::Borrow;

// ----------------------------------------------------

fn main() -> PanicResult<()> {
    _main_print_reruns()?;

    let meta = _main_link_library()?;

    _main_gen_bindings(meta)?;

    Ok(())
}

fn _main_link_library() -> PanicResult<BuildMeta> {
    match ::env::mode() {
        Mode::Auto => {
            if let Ok(meta) = probe::probe_and_link() {
                return Ok(meta);
            } else {
                Ok(build::build_from_source_and_link()?)
            }
        },
        Mode::BuildOnly => Ok(build::build_from_source_and_link()?),
        Mode::SystemOnly => Ok(probe::probe_and_link()?),
    }
}

// ----------------------------------------------------

// Information discovered during the build that is
// needed during bindgen.
struct BuildMeta {
    // "lammps/library.h" or similar. (It is not available under that path when building directly
    //  from source, so some adjustments are needed)
    header: &'static str,
    // A bunch of -I arguments
    include_dirs: CcFlags,
    // A bunch of -D arguments
    defines: CcFlags,
}

// ----------------------------------------------------

fn _main_gen_bindings(meta: BuildMeta) -> PanicResult<()> {
    let BuildMeta { header, include_dirs, defines } = meta;

    let out_path = PathDir::new(env::expect("OUT_DIR"))?;

    let _ = ::std::fs::create_dir(out_path.join("codegen"));

    let mut gen = ::bindgen::Builder::default();
    gen = gen.header_contents(
        "include_lammps.h",
        &format!(r##"#include <{}>"##, header),
    );

    // Ensure that the header contains the right features corresponding
    // to what was enabled (e.g. `LAMMPS_EXCEPTIONS`).
    gen = gen.clang_args(defines.to_args());
    gen = gen.clang_args(include_dirs.to_args());

    // support older versions of libclang, which will mangle even
    // the names of C functions unless we disable this.
    gen = gen.trust_clang_mangling(false);
    gen = gen.whitelist_function("lammps.*");

    gen.generate()
        .expect("Unable to generate bindings for 'lammps'!")
        .write_to_file(out_path.join("codegen/lammps.rs"))
        .expect("Couldn't write bindings for 'lammps'!");

    Ok(())
}

// ----------------------------------------------------

fn _main_print_reruns() -> PanicResult<()> {
    // We won't print rerun directives for things in 'lammps' because there's a lot of files
    // there and you shouldn't be touching it anyways.
    //
    // ...but we will rebuild in response to checking out a new commit for the submodule.
    let git_dir = build::lammps_dotgit_dir()?;
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

pub enum Mode {
    Auto,
    SystemOnly,
    BuildOnly,
}

mod env {
    #[allow(unused_imports)]
    use super::*;
    use ::std::env;

    pub fn mode() -> Mode {
        let var = "RUST_LAMMPS_SOURCE";
        let value = get_rerun_nonempty(var).unwrap_or_else(|| String::from("auto"));
        match &value[..] {
            "auto" => Mode::Auto,
            "system" => Mode::SystemOnly,
            "build" => Mode::BuildOnly,
            s => panic!("Bad value for RUST_LAMMPS_SOURCE: {}", s)
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

// A flag for the C compiler (or preprocessor or linker).
#[derive(PartialEq, Eq)]
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
    // an option, because there's no reliable way to tell.
    //
    // This will only cause trouble if an unrecognized option is given
    // an option argument beginning with -I/-l/-L/-D or similar, and
    // they are separated by a space.  This seems unlikely.
    Other(String),
}

pub struct CcFlags(Vec<CcFlag>);

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
//
// This format is required for `cargo:rustc-flags`.
#[allow(unused)]
struct WithSpace<C>(C);
impl<C> fmt::Display for WithSpace<C> where C: Borrow<CcFlag> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    { self.0.borrow().fmt_with_space(" ", f) }
}

// Displays as "-liberty"
//
// This format is convenient for producing atomic arguments without fear
// of quoting issues.
struct WithoutSpace<C>(C);
impl<C> fmt::Display for WithoutSpace<C> where C: Borrow<CcFlag> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    { self.0.borrow().fmt_with_space("", f) }
}

impl CcFlags {
    fn to_args(&self) -> Vec<String> {
        self.0.iter().map(|x| WithoutSpace(x).to_string()).collect()
    }
}

// ----------------------------------------------------

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
