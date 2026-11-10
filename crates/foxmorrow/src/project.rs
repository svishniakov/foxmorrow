use std::path::{Path, PathBuf};

use foxmorrow_core::catalog::CATALOG_FILE;
use foxmorrow_core::{Error, Result, baseline};

pub fn for_init(explicit: Option<PathBuf>, cwd: &Path) -> PathBuf {
    explicit.unwrap_or_else(|| baseline::toplevel(cwd).unwrap_or_else(|_| cwd.to_path_buf()))
}

pub fn for_existing(explicit: Option<PathBuf>, cwd: &Path) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    cwd.ancestors()
        .find(|dir| dir.join(".foxmorrow").join("current").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            Error::NotFound(
                "no foxmorrow run in this directory or its parents; start one with foxmorrow init"
                    .into(),
            )
        })
}

pub fn plugin_root(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) =
        explicit.or_else(|| std::env::var_os("FOXMORROW_PLUGIN_ROOT").map(PathBuf::from))
    {
        return Ok(path);
    }
    let executable = std::env::current_exe()?;
    executable
        .ancestors()
        .find(|dir| dir.join(CATALOG_FILE).is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            Error::NotFound(
                "foxmorrow plugin root; set FOXMORROW_PLUGIN_ROOT or pass --plugin-root".into(),
            )
        })
}
