use ::{BuildMeta, CcFlag, CcFlags};

pub(crate) fn probe_and_link() -> Result<BuildMeta, ::pkg_config::Error> {
    probe_and_link_via_pkgconfig()
}

// Lammps does offer a cmake-based build system, which appears to be designed to install a
// `.pc` file for pkgconfig.  We can look for that.
fn probe_and_link_via_pkgconfig() -> Result<BuildMeta, ::pkg_config::Error> {
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

    Ok(BuildMeta {
        // The CMakeFile thankfully appears to install the header under a sane, unambiguous path.
        // (fortituously the same one chosen by lammps-sys 0.3.x!)
        header: "lammps/library.h",
        include_dirs,
        defines,
    })
}
