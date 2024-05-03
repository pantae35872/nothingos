use std::{
    env,
    process::{Command, Stdio},
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let curr_dir = env::current_dir().expect("no current dir");
    if let Some(test_kernel) = args.get(1) {
        let root_dir = curr_dir.parent().unwrap().parent().unwrap();
        let mut mv = Command::new("mv");
        let mut makefile_build = Command::new("make");
        let mut makefile_run = Command::new("make");
        mv.arg(test_kernel)
            .arg(root_dir.join("kernel.bin"))
            .current_dir(&curr_dir);
        makefile_build
            .arg("maker-no-kernel")
            .current_dir(&root_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        makefile_run
            .arg("test-run")
            .current_dir(&root_dir)
            .stderr(Stdio::null());
        mv.status().expect("Could not move");
        makefile_build.status().expect("make file failed");
        makefile_run.status().expect("run failed");
    }
}
