mod keyed;
pub mod store;

use std::io;
use std::path::Path;

pub fn create_parent_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
    } else {
        Ok(())
    }
}
