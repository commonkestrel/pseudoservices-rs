use std::process::Command;

fn main() {
    let success = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "tsc", "--project", "../ts"])
            .status()
            .expect("failed to execute `tsc`")
            .success()
    } else {
        Command::new("tsc")
            .args(["--project", "../ts"])
            .status()
            .expect("failed to execute `tsc`")
            .success()
    };

    if !success {
        panic!("failed to build typescript with `tsc`");
    }

    println!("cargo:rerun-if-changed=../ts")
}