use std::fs::create_dir_all;
use std::path::PathBuf;
// Thank you https://stackoverflow.com/a/27582993
// (some modifications to please the type checker)
#[macro_export]
macro_rules! collection {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {{
        core::convert::From::from([$(($k.into(), $v.into()),)*])
    }};
    // set-like
    ($($v:expr),* $(,)?) => {{
        core::convert::From::from([$($v.into(),)*])
    }};
}

pub fn create_parent(path: &PathBuf) -> std::io::Result<()> {
    let parent = path.parent();

    if let Some(parent) = parent {
        if !parent.exists() {
            return create_dir_all(parent)
        }
    }

    Ok(())
}