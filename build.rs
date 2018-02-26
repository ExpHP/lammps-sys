extern crate make_cmd;
extern crate num_cpus;

extern crate bindgen;
extern crate path_abs; // better error messages
extern crate walkdir;
#[macro_use] extern crate extension_trait;

use ::path_abs::{PathArc, PathDir, PathFile, FileRead, FileWrite};
use ::path_abs::{Result as PathResult, Error as PathError};
type BoxResult<T> = Result<T, Box<std::error::Error>>;
use ::walkdir::WalkDir;

use ::std::path::Path;
use ::std::process::{Command, Stdio};
use ::std::io::Result as IoResult;
use ::std::io::BufReader;
use ::std::fmt::Display;
use ::std::io::prelude::*;

// ----------------------------------------------------
// "Constants". Sorta.
// In any case, these require continued maintenence so that they
// accurately reflect the directory structure.

fn lammps_git_dir() -> PathResult<PathDir> {
    PathDir::new(".git/modules/lammps")
}

fn lammps_repo_dir() -> PathResult<PathDir> {
    const PATH: &'static str = "lammps";
    // This library might do bad things if lmp_dir is a symlink,
    // due to path canonicalization...
    assert!(!PathArc::new(PATH).symlink_metadata()?.file_type().is_symlink());
    PathDir::new(PATH)
}

// ----------------------------------------------------
// macros

macro_rules! vec_from_features {
    ($( $feat:expr => $expr:expr, )*) => {{
        #[allow(unused_mut)]
        let mut vec = vec![];
        $( #[cfg(feature = $feat)] { vec.push($expr); })*
        vec
    }};
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
    lammps_defines: Vec<String>,
}

fn _main_do_static_build() -> PanicResult<BuildMeta> {
    let lmp_dir = LammpsDir::get()?;
    let orig_name = env::make();
    let orig_path = lmp_dir.find_prepackaged_makefile(&orig_name)?
        .unwrap_or_else(|| panic!("Makefile for '{}' not found!", orig_name));

    make::so_clean_its_like_its_not_even_there(&lmp_dir)?;

    // Make a custom makefile with the right C preprocessor defines,
    // and record them.
    let lammps_defines = {
        let makefile = LammpsMakefile::from_reader(BufReader::new(orig_path.read()?))?;

        let mut defs = makefile.lmp_defines();
        defs.append(&mut vec_from_features![
            "exceptions" => "-DLAMMPS_EXCEPTIONS".into(),
            "bigbig"     => "-DLAMMPS_BIGBIG".into(),
            // TODO: scout the LAMMPS docs/codebase for more
        ]);

        let makefile = makefile.with_lmp_defines(defs);

        let file = lmp_dir.create_mine_makefile("rust")?;
        makefile.to_writer(file)?;

        makefile.all_inc_flags()
    };

    vec_from_features![
        "user-misc" => "yes-user-misc",
        "user-omp"  => "yes-user-omp",
        // TODO: scout the LAMMPS docs/codebase for more
    ].into_iter().for_each(|target: &str| {
        ::make::jay(&lmp_dir).arg(target).run_custom().unwrap();
    });

    ::make::run_fast_and_loose(
        &lmp_dir,
        |c| c.arg("rust").arg("mode=lib"),
    ).unwrap();

    // NOTE: I'd like to verify that the library exists, but cannot
    //       easily do so in a cross-platform manner
    println!("cargo:rust-link-search={}", lmp_dir.join("src").display());
    println!("cargo:rust-link-lib=static=lammps");

    Ok(BuildMeta { lammps_defines })
}

// ----------------------------------------------------

fn _main_gen_bindings(meta: BuildMeta) -> PanicResult<()> {
    let BuildMeta { lammps_defines } = meta;
    
    let lmp_dir = LammpsDir::get()?;
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
        gen = gen.clang_args(&lammps_defines);
        
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
    let git_dir = lammps_git_dir()?;
    assert!(git_dir.join("HEAD").exists());
    rerun_if_changed(git_dir.join("HEAD").display());
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

extension_trait!{
    CommandExt for Command {
        fn run_custom(&mut self) -> PanicResult<()> {
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

    pub fn so_clean_its_like_its_not_even_there(lmp_dir: &PathDir) -> PanicResult<()> {
        // Basically:
        //
        //     rm -rf source/!(.git)
        //     (cd source && git reset --hard)
        //
        // because lammps' own "make clean-all" simply does not even
        // come close to cutting it.

        let good_dotgit = lmp_dir.join(".git");
        let dotgit = PathFile::new(&good_dotgit)?.copy("temp-source.git")?;

        // temporarily trap errors until .git is back in place
        (|| {
            lmp_dir.clone().remove_all()?;
            PathDir::create(lmp_dir)?;
            dotgit.clone().rename(&good_dotgit).unwrap(); // FIXME rm PanicError
            Ok(())
        })().map_err(|e: PathError| {
            eprintln!(
                "CAUTION: \
                An error occured while the lammps submodule's .git was displaced! \
                You may need to resolve this yourself. Sorry!"
            );
            eprintln!("   It was moved to: {}", dotgit.display());
            eprintln!("   It belongs at:   {}", good_dotgit.display());
            e
        })?;

        Command::new("git")
            .args(&["reset", "HEAD", "--hard"])
            .current_dir(&lmp_dir)
            .run_custom()?;

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

    pub fn make() -> String {
        get_nonempty("RUST_LAMMPS_MAKE").unwrap_or_else(|| "serial".into())
    }

    pub fn expect(var: &str) -> String {
        env::var(var)
           .unwrap_or_else(|e| panic!("error reading {}: {}", var, e))
    }

    fn get_nonempty(s: &str) -> Option<String> {
        env::var(s).map(Some).unwrap_or_else(|e| match e {
            env::VarError::NotPresent => None,
            env::VarError::NotUnicode(e) => panic!("var {} is not unicode: {:?}", s, e),
        // }).filter(|s| s != "") // not stable
        }).and_then(|s| match &s[..] {
            "" => None,
            _ => Some(s),
        })
    }
}

// ----------------------------------------------------

struct LammpsDir(PathDir);
impl ::std::ops::Deref for LammpsDir {
    type Target = PathDir;
    fn deref(&self) -> &PathDir { &self.0 }
}

impl LammpsDir {
    pub fn get() -> PathResult<Self> {
        lammps_repo_dir().map(LammpsDir)
    }
    pub fn find_prepackaged_makefile(&self, name: &str) -> PathResult<Option<PathFile>> {
        let full_name = format!("Makefile.{}", name);
        assert_eq!(Path::new(&full_name).components().count(), 1);

        let subdirs = &[
            "src/MAKE",
            "src/MAKE/OPTIONS",
            "src/MAKE/MACHINE",
        ];
        for d in subdirs {
            // (dir is constructed simply to Err on a missing directory)
            let dir = PathDir::new(self.join(d))?;
            if let Ok(path) = PathFile::new(dir.join(&full_name)) {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }

    /// For creating thine makefile.
    pub fn create_mine_makefile(&self, name: &str) -> PathResult<FileWrite> {
        let make = PathDir::new(self.join("src/MAKE"))?; // should exist

        let mine = make.join("MINE"); // might not exist yet
        let _ = ::std::fs::create_dir(&mine);
        let mine = PathDir::new(mine)?;

        FileWrite::create(mine.join(format!("Makefile.{}", name)))
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

        // Get all the "-D" flags. ("-D" included)
        pub fn lmp_defines(&self) -> Vec<String> {
            self.var_def("LMP_INC")
                .expect("could not locate LMP_INC definition in makefile")
                .get().split_whitespace().map(|s| s.into()).collect()
        }

        // Get all the "-D" flags and "-I" flags for headers.
        // These are communicated to bindgen.
        // 
        // Purposes:
        // - communicate flags like LAMMPS_EXCEPTIONS to bindgen
        // - help bindgen find the header files for the MPI STUBS library
        //   in serial builds of LAMMPS.
        pub fn all_inc_flags(&self) -> Vec<String> {
            let mut includes = vec![];
            for var in vec!["LMP_INC", "MPI_INC", "FFT_INC", "JPG_INC"] {
                includes.extend({
                    self.var_def(var)
                        .expect(&format!("could not locate {} definition in makefile", var))
                        .get().split_whitespace().map(|s| s.into())
                });
            }
            includes
        }

        pub fn with_lmp_defines<Ss>(mut self, defs: Ss) -> Self
        where Ss: IntoIterator, Ss::Item: Into<String>,
        {
            // validate defs
            let defs = defs.into_iter().map(|s| s.into()).inspect(|s| {
                let mut words = s.split_whitespace();
                let word = words.next().unwrap();
                assert!(word.len() > 0);
                assert!(word.starts_with("-D"));
                assert!(words.next().is_none());
            });

            self.var_def_mut("LMP_INC")
                .expect("could not locate LMP_INC definition in makefile")
                .set(&defs.join(" "));
            self
        }

        // get a handle for reading a simple variable assignment
        pub fn var_def(&self, name: &str) -> Option<VarDef> {
            self._var_def_data(name).map(|(line, start_col)| {
                VarDef_ { makefile: self, line, start_col }
            })
        }

        pub fn var_def_mut(&mut self, name: &str) -> Option<VarDefMut> {
            self._var_def_data(name).map(|(line, start_col)| {
                VarDef_ { makefile: self, line, start_col }
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
                /*.inspect(|&(_, line)| {
                    eprintln!("{:?} {} {} {}", line,
                        line.starts_with(name),
                        line.len() > name.len(),
                        !is_identifier_char(line.as_bytes()[name.len()]),
                    );
                })*/
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
        pub fn get(&self) -> &str {
            let VarDef_ { ref makefile, line, start_col } = *self;
            &(**makefile).0[line][start_col..]
        }
    }

    impl<T: ::std::ops::DerefMut<Target=LammpsMakefile>> VarDef_<T> {
        /// Write a value for the variable
        pub fn set<S: AsRef<str>>(&mut self, s: S) {
            let s = s.as_ref();

            assert!(!s.ends_with("\\")); // what are you trying to pull?

            let VarDef_ { ref mut makefile, line, start_col } = *self;
            let line = &mut (**makefile).0[line];
            line.truncate(start_col);
            *line += s;
            eprintln!("LINE: {:?}", line);
        }
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

