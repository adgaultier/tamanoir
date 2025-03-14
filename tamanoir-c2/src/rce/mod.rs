pub mod builder;
pub mod tester;
use std::{
    io::{self, Write},
    process::Command,
};

pub struct Cmd {
    pub shell: String,
    pub stdout: bool,
}

impl Cmd {
    pub fn exec(&self, cmd: String) -> Result<(), String> {
        let mut program = Command::new(&self.shell);
        let prog: &mut Command = program.arg("-c").arg(&cmd);

        let output = prog
            .output()
            .map_err(|_| format!("Failed to run {}", cmd))?;
        if self.stdout {
            io::stdout().write_all(&output.stdout).unwrap();
            io::stderr().write_all(&output.stderr).unwrap();
        }
        if !output.status.success() {
            return Err(format!(
                "{} failed with status {}: {}",
                cmd,
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
}
