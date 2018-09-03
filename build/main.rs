extern crate make_cmd;
extern crate num_cpus;

extern crate bindgen;
extern crate path_abs; // better error messages
extern crate walkdir;
extern crate pkg_config;
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

use ::path_abs::{PathArc, PathDir, PathFile, FileRead};
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
    if let Ok(meta) = probe::probe_and_link() {
        return Ok(meta);
    } else {
        // ignore errors
    }

    let meta = build::build_from_source_and_link()?;
    Ok(meta)
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

    // Make a bindgen builder with flags shared by both invocations.
    // (these things don't implement Clone...)
    let make_gen = || {
        let mut gen = ::bindgen::Builder::default();

        // Lammps' poorly-named header file...
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
        gen
    };

    make_gen()
        .whitelist_function("lammps.*")
        .generate()
        .expect("Unable to generate bindings for 'lammps'!")
        .write_to_file(out_path.join("codegen/lammps.rs"))
        .expect("Couldn't write bindings for 'lammps'!");

    Ok(())
}

// ----------------------------------------------------

fn _main_print_reruns() -> PanicResult<()> {
    // Because we clean 'source' by deleting *literally everything*, there's no point
    // in checking it for any changes. Only the checked-out commit hash matters.
    let git_dir = build::lammps_dotgit_dir()?;
    assert!(git_dir.join("HEAD").exists());
    rerun_if_changed(git_dir.join("HEAD").display());

    rerun_if_changed("Cargo.toml");
    rerun_if_changed_recursive("src".as_ref())?;

    let file = BufReader::new(FileRead::read("build-data/rerun-if-env-changed")?);
    read_simple_lines(file, "##")?.into_iter().for_each(rerun_if_env_changed);
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

mod env {
    #[allow(unused_imports)]
    use super::*;
    use ::std::env;

    pub fn makefile() -> PathFile {
        let var = "RUST_LAMMPS_MAKEFILE";
        match get_rerun_nonempty(var) {
            None => {
                let path = build::lammps_repo_dir().join("src/MAKE/Makefile.serial");
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
