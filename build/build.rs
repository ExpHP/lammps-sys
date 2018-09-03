// automated builds of lammps from source

use ::{BoxResult, PanicResult};
use ::{BuildMeta, CcFlag, CcFlags};
use ::std::process::{Command, Stdio};
use ::path_abs::{PathArc, PathDir};

const SUBMODULE_PATH: &'static str = "lammps";

// ----------------------------------------------------

/// Build lammps from source and emit linker flags
pub(crate) fn build_from_source_and_link() -> PanicResult<BuildMeta> {
    let lmp_dir = lammps_repo_dir();

    let mut cmake = ::cmake::Config::new(lammps_cmake_root()?);
    let mut defines = CcFlags(vec![]);

    cmake.define("BUILD_LIB", "yes");
    cmake.define("BUILD_SHARED_LIBS", "no");
    cmake.define("BUILD_EXE", "no");

    for key in ::packages::cmake_flags_from_features() {
        cmake.define(key, "yes");
    }

    if cfg!(feature = "exceptions") {
        cmake.define("LAMMPS_EXCEPTIONS", "yes");
        defines.0.push(CcFlag::Define("LAMMPS_EXCEPTIONS".into()));
    }

    let lib_dir = PathDir::new(cmake.build())?;

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=lammps");

    // FIXME: Does this cause problems for other crates that need libstdc++?
    //        Should there be a stdcpp-sys crate just for this?
    // NOTE: This is only needed for static builds.
    println!("cargo:rustc-flags=-l stdc++");

    Ok(BuildMeta {
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

        path = PathArc::new(path.parent().unwrap()).join(line.trim()).canonicalize()?;
    }
    Ok(PathDir::new(path)?)
}

/// Path to the directory within the lammps submodule that contains CMakeLists.txt.
pub(crate) fn lammps_cmake_root() -> BoxResult<PathDir> {
    Ok(PathDir::new(lammps_repo_dir().join("cmake").canonicalize()?)?)
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
