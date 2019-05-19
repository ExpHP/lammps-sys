// automated builds of lammps from source

use ::{BoxResult, PanicResult};
use ::{BuildMeta, CcFlag, CcFlags};
use ::std::process::{Command, Stdio};
use ::std::path::Path;
use ::path_abs::{PathArc, PathDir, PathFile};
use ::walkdir::WalkDir;

const SUBMODULE_PATH: &'static str = "lammps";

// ----------------------------------------------------

/// Build lammps from source and emit linker flags
pub(crate) fn build_from_source_and_link() -> PanicResult<BuildMeta> {
    let lmp_dir = lammps_repo_dir_build_copy()?;

    let mut cmake = ::cmake::Config::new(lammps_cmake_root()?);
    let mut defines = CcFlags(vec![]);
    let mut include_dirs = CcFlags(vec![]);

    // Override the value from `include(GNUInstallDirs)` (which might be lib or lib64 or etc.)
    // with a fixed destination for easier linking.
    cmake.define("CMAKE_INSTALL_LIBDIR", "lib");

    cmake.define("BUILD_LIB", "yes");

    // NOTE: Building shared because I don't trust the static builds to work.
    //
    // (see the note below about libraries appearing in CMAKE_INSTALL_PREFIX/build, of all places--
    //  and when I went to go build a static LAMMPS on my own just to investigate, it successfully
    //  installed no library at all! Nothing but an `/etc` and a `/share`!
    //
    //  NOTE: This was all on stable_22Aug2018, and might be fixed on patch_31Aug2018;
    //  I haven't tested yet.)
    cmake.define("BUILD_SHARED_LIBS", "yes");
    cmake.define("BUILD_EXE", "no");

    for key in ::packages::cmake_flags_from_features() {
        cmake.define(key, "yes");
    }

    cmake.define("CMAKE_RULE_MESSAGES:BOOL", "OFF");
    cmake.define("CMAKE_VERBOSE_MAKEFILE:BOOL", "ON");

    cmake.define("BUILD_MPI", match cfg!(feature = "mpi") {
        true => "yes",
        false => "no",
    });

    if cfg!(feature = "exceptions") {
        cmake.define("LAMMPS_EXCEPTIONS", "yes");
        defines.0.push(CcFlag::Define("LAMMPS_EXCEPTIONS".into()));
    }

    let lib_dir = PathDir::new(cmake.build())?;
    println!("cargo:rustc-link-search=native={}/lib", lib_dir.display());
    println!("cargo:rustc-link-lib=lammps");

//  // FIXME: Does this cause problems for other crates that need libstdc++?
//  //        Should there be a stdcpp-sys crate just for this?
//  // NOTE: This is only needed for static builds.
//    println!("cargo:rustc-flags=-l stdc++");

    include_dirs.0.push(CcFlag::IncludeDir(lmp_dir.into()));
    Ok(BuildMeta {
        header: "src/library.h",
        include_dirs,
        defines,
    })
}

// ----------------------------------------------------

/// HACK:
/// See https://users.rust-lang.org/t/cargo-exclude-all-contents-of-a-directory-but-keep-the-directory/28137
///
/// Create a dedicated copy of the lammps directory for building.
///
/// The copy will be modified in such a way to ensure that the build succeeds.
///
/// **None of this should be necessary once there is a way to specify in cargo that
/// a directory should be preserved while its contents are ignored.**
fn lammps_repo_dir_build_copy() -> BoxResult<PathDir> {
    let src_dir = lammps_repo_dir();

    let copy_path = ::env::out_dir().join("lammps");
    if copy_path.exists() {
        return Ok(PathDir::new(copy_path)?);
    };

    let copy = PathDir::create(::env::out_dir().join("lammps"))?;

    let suffix = |entry: &walkdir::DirEntry| {
        entry.path().strip_prefix(&src_dir).unwrap_or_else(|e| panic!("{}", e)).to_owned()
    };

    // Make the copy.
    let walker = {
        WalkDir::new(&src_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(|entry| {
                // Save space on local builds by filtering out some of the worst offenders
                // on the exclude list from Cargo.toml.
                // (this does nothing for builds from crates.io)
                let blacklist = &[
                    Path::new("examples"),
                    Path::new("bench"),
                    Path::new("doc/src"),
                    Path::new("doc/util"),
                    Path::new("tools"),
                    Path::new("potentials"),
                ];

                !blacklist.contains(&suffix(&entry).as_ref())
            })
    };
    for entry in walker {
        let entry = entry?;
        let dest = copy.join(suffix(&entry));

        let ty = entry.file_type();
        if ty.is_file() {
            PathFile::new(entry.path()).unwrap_or_else(|e| panic!("{}", e)).copy(dest)?;
        } else if ty.is_dir() {
            PathDir::create(dest)?;
        }
    }

    // And now for the whole entire point of this function:
    //
    // The potentials/ directory needs to exist, but it does not exist
    // in the packaged crate file on crates.io.
    PathDir::create(copy.join("potentials"))?;

    Ok(copy)
}

pub(crate) fn lammps_repo_dir() -> PathDir {
    // This library might do bad things if lmp_dir is a symlink,
    // due to path canonicalization...
    let msg = "Could not find lammps submodule";
    assert!(!PathArc::new(SUBMODULE_PATH).symlink_metadata().expect(msg).file_type().is_symlink());
    PathDir::new(SUBMODULE_PATH).expect(msg)
}

/// Path to the .git directory for the lammps submodule, if there is one
pub(crate) fn lammps_dotgit_dir() -> BoxResult<Option<PathDir>> {
    // HACK: git submodules handled normally have a ".git file"
    //       containing the path to the true .git.
    //       ...but cargo does not handle submodules normally when the
    //       crate is built as an external dependency, so we must be
    //       equipped to handle both cases.
    //
    // We need the raw one here (not the copy in OUT_DIR) so we can correctly interpret
    // relative paths.
    let path = lammps_repo_dir().join(".git");
    if !path.exists() {
        // 'cargo vendor' doesn't even put a .git there
        return Ok(None);
    }

    let mut path = path.canonicalize()?;
    while path.is_file() {
        let text = ::std::fs::read_to_string(&path)?;
        let line = text.lines().next().expect("empty .git file!");

        assert!(text.starts_with("gitdir:"));
        let line = &line["gitdir:".len()..];

        path = PathArc::new(path.parent().unwrap()).join(line.trim()).canonicalize()?;
    }
    Ok(Some(PathDir::new(path)?))
}

/// Path to the directory within the lammps submodule that contains CMakeLists.txt.
pub(crate) fn lammps_cmake_root() -> BoxResult<PathDir> {
    let cmake_root = lammps_repo_dir_build_copy()?.join("cmake");
    Ok(PathDir::new(cmake_root.canonicalize().map_err(|_| {
        format!("could not resolve {:?}, you probably forgot to `git submodule update --init`\n"
                + "note that you might need to run cargo clean after doing so.", cmake_root)
    })?)?)
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
