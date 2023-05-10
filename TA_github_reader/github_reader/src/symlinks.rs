use std::{env, ffi::OsStr, os::unix::fs::symlink, path::Path};

fn current_exe() -> Option<String> {
    env::current_exe()
        .ok()
        .as_ref()
        .map(Path::new)
        .and_then(Path::to_str)
        .map(String::from)
}

// fn current_exe_name() -> Option<String> {
//     env::current_exe()
//         .ok()
//         .as_ref()
//         .map(Path::new)
//         .and_then(Path::file_name)
//         .and_then(OsStr::to_str)
//         .map(String::from)
// }

fn current_exe_parent() -> Option<String> {
    env::current_exe()
        .ok()
        .as_ref()
        .map(Path::new)
        .and_then(Path::parent)
        .and_then(Path::to_str)
        .map(String::from)
}

pub fn current_exe_from_args() -> Option<String> {
    env::args()
        .next()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from)
}

pub fn make_symlinks() {
    // TODO read symlinks to check the correct ones exist before creating them
    let source = current_exe().unwrap();
    let target = format!("{}/azure_client", current_exe_parent().unwrap());
    symlink(source, target).ok();
}
