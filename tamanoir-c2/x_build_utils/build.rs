use std::process::Command;

#[cfg(target_arch = "x86_64")]
fn build_x86_64() {
    let binary_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME not set");
    let base_path = "target/x86_64-unknown-linux-gnu/release";
    let elf_path = format!("{}/{}", base_path, binary_name);
    let bin_path = format!("{}/{}_x86_64.bin", base_path, binary_name);

    let output = Command::new("x86_64-linux-gnu-strip")
        .arg("-s")
        .arg("--strip-unneeded")
        .arg(&elf_path)
        .output()
        .expect("Failed to run strip");

    if !output.status.success() {
        panic!(
            "strip failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = Command::new("x86_64-linux-gnu-objcopy")
        .arg("-O")
        .arg("binary")
        .arg(&elf_path)
        .arg(&bin_path)
        .output()
        .expect("Failed to run objcopy");
    if !output.status.success() {
        panic!(
            "objcopy failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

fn main() {
    #[cfg(target_arch = "x86_64")]
    build_x86_64()
}
