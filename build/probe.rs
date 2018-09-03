use ::{BuildMeta, CcFlag, CcFlags};

pub(crate) fn probe_and_link() -> Result<BuildMeta, ProbeError> {
    probe_and_link_via_pkgconfig()
}

pub(crate) enum ProbeError {
    PkgConfig(::pkg_config::Error),
    String(String),
}

impl From<::pkg_config::Error> for ProbeError {
    fn from(e: ::pkg_config::Error) -> Self {
        ProbeError::PkgConfig(e)
    }
}

// Lammps does offer a cmake-based build system, which appears to be designed to install a
// `.pc` file for pkgconfig.  We can look for that.
fn probe_and_link_via_pkgconfig() -> Result<BuildMeta, ProbeError> {
    let library = ::pkg_config::probe_library("liblammps")?;
    let include_dirs = CcFlags({
        library.include_paths.into_iter()
            .map(Into::into).map(CcFlag::IncludeDir)
            .collect()
    });
    let defines = CcFlags({
        library.defines.into_iter()
            .map(|(key, value)| match value {
                Some(value) => CcFlag::Define(format!("{}={}", key, value)),
                None => CcFlag::Define(format!("{}", key)),
            })
            .collect()
    });

    if cfg!(feature = "exceptions") {
        // NOTE: shoving subtleties like "-DLAMMPS_EXCEPTIONS=definition" under the rug.
        let needle = CcFlag::Define(String::from("LAMMPS_EXCEPTIONS"));
        if !defines.0.iter().any(|x| x == &needle) {
            let msg = String::from(r#"\
                system lammps was built without -DLAMMPS_EXCEPTIONS \
                (--features=exceptions)\
            "#);
            return Err(ProbeError::String(msg));
        }
    }

    Ok(BuildMeta {
        // The CMakeFile thankfully appears to install the header under a sane, unambiguous path.
        // (fortituously the same one chosen by lammps-sys 0.3.x!)
        header: "lammps/library.h",
        include_dirs,
        defines,
    })
}
