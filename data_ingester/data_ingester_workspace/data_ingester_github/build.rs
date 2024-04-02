//! build.rs
//!
//! Set an environment variable with the current git commit hash. Used in logging
use std::process::Command;

fn main() {
    // note: add error checking yourself.
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("Should be able to get current GIT commit hash");

    let git_hash = String::from_utf8(output.stdout).expect("Git hash should be valid UTF8");
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
