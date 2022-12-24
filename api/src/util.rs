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

pub fn plural_s(n: usize) -> &'static str {
    return if n == 1 { "" } else { "s" }
}
pub fn plural_s_opposite(n: usize) -> &'static str {
    return if n == 1 { "s" } else { "" }
}

pub fn nice_list<'a, I, S>(iter: I, last_join: S) -> String where I: IntoIterator, I::Item: ToString, S: ToString {
    let collected: Vec<String> = iter.into_iter()
        .map(|v| v.to_string())
        .collect();

    match collected.len() {
        0 => "".into(),
        1 => collected.first().unwrap().clone(),
        2 => {
            let first = collected.first().unwrap();
            let last = collected.last().unwrap();
            format!("{} {} {}", first, last_join.to_string(), last)
        },
        length => {
            let mut str = String::new();
            for (i, item) in collected.iter().rev().enumerate() {
                let prefix = if i == length - 1 {
                    "".into()
                } else {
                    if i == 0 {
                        format!(", {} ", last_join.to_string())
                    } else {
                        ", ".into()
                    }
                };
                str.push_str(prefix.as_str());
                str.push_str(item.as_str());
            }
            str
        }
    }
}