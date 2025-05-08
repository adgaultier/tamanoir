pub mod utils;

use std::{
    env,
    fs::{self, File},
    io::Write,
    str::FromStr,
};

use home::home_dir;
use log::{info, log_enabled, Level};
use tamanoir_common::{Engine, TargetArch};
use tempfile::{Builder, TempDir};
use utils::{
    cross_build_base_cmd, format_build_vars_for_cross, init_utils_files, parse_package_name,
    UTILS_FILES,
};

use crate::rce::Cmd;

pub fn build(
    crate_path: String,
    engine: Engine,
    target: TargetArch,
    build_vars: String,
) -> Result<(), String> {
    let current_arch = env::consts::ARCH;

    let should_x_compile = TargetArch::from_str(current_arch).unwrap() != target;
    let mut build_dir = home_dir().unwrap();
    build_dir.push(".tamanoir/bins");
    if !build_dir.exists() {
        fs::create_dir_all(build_dir.clone()).map_err(|e| {
            format!(
                "couldn't create build directory ({}): {}",
                build_dir.display(),
                e
            )
        })?;
    }
    let tmp_dir = Builder::new()
        .prefix("tamanoir-rce")
        .tempdir()
        .map_err(|_| "Error creating temp dir")?;
    let out_dir = format!("{}", build_dir.display());
    if should_x_compile {
        x_compile(
            engine,
            crate_path.clone(),
            target,
            build_vars,
            tmp_dir,
            out_dir,
        )?
    } else {
        compile(crate_path, build_vars, tmp_dir, out_dir)?
    };

    Ok(())
}
pub fn x_compile(
    engine: Engine,
    crate_path: String,
    target: TargetArch,
    build_vars: String,
    tmp_dir: TempDir,
    out_dir: String,
) -> Result<(), String> {
    let cmd = Cmd {
        shell: "/bin/bash".into(),
        stdout: log_enabled!(Level::Debug),
    };
    init_utils_files()?;
    let build_vars_formatted = format_build_vars_for_cross(build_vars)?;
    let bin_name = parse_package_name(crate_path.clone())?;
    let tmp_path = tmp_dir.path().to_string_lossy().to_string();
    info!("installing cross-rs");
    let cmd0 = format!(
        "cargo install cross --git https://github.com/cross-rs/cross --rev 36c0d78; cp  -ar {crate_path}/. {tmp_path}"
    ); // we cannot use a released version yet (see https://github.com/cross-rs/cross/issues/1498#issuecomment-2133001860) so we're stuck on the main branch
       // it is annoying to reinstall each time so I fix a sha @ 2025-03-13
    cmd.exec(cmd0)?;
    info!("start x compilation with cross to target {}", target);
    if let Some(cross_conf) = UTILS_FILES
        .get()
        .unwrap()
        .get(&format!("Cross_{target}.toml"))
        .cloned()
    {
        let out_path = format!("{tmp_path}/Cross.toml");
        File::create(&out_path)
            .map_err(|_| format!("Couldn't create {}", &out_path))?
            .write_all(cross_conf.as_bytes())
            .map_err(|_| format!("Couldn't create {}", &out_path))?;
    }

    let cmd1 = cross_build_base_cmd(
        tmp_path.clone(),
        engine,
        build_vars_formatted,
        target.clone(),
    );
    cmd.exec(cmd1.clone())?;

    info!("run post install scripts with cross");
    let post_build_script = UTILS_FILES.get().unwrap().get("build.rs").cloned().unwrap();
    let out_path = format!("{tmp_path}/build.rs");
    File::create(&out_path)
        .map_err(|_| format!("Couldn't create {}", &out_path))?
        .write_all(post_build_script.as_bytes())
        .map_err(|_| format!("Couldn't create {}", &out_path))?;
    cmd.exec(cmd1)?;

    let cmd3 = format!(
        "cp  {tmp_path}/target/{target}-unknown-linux-gnu/release/{bin_name}_{target}.bin {out_dir}/{bin_name}_{target}.bin"
    );
    cmd.exec(cmd3)?;

    Ok(())
}

pub fn compile(
    crate_path: String,
    build_vars: String,
    tmp_dir: TempDir,
    out_dir: String,
) -> Result<(), String> {
    let bin_name = parse_package_name(crate_path.clone())?;
    let cmd = Cmd {
        shell: "/bin/bash".into(),
        stdout: log_enabled!(Level::Debug),
    };

    info!("start compilation of {}", bin_name);
    let tmp_path = tmp_dir.path().to_string_lossy();
    let cmd0 = format!(
        "cp -ar {crate_path}/. {tmp_path} && cd {tmp_path} && {build_vars}  cargo build --release"
    );
    cmd.exec(cmd0)?;

    info!("start post-build operations");
    let cmd1 = format!(
        "strip -s --strip-unneeded {tmp_path}/target/release/{bin_name}"
    );
    let cmd2 = format!(
        "objcopy -O binary {}/target/release/{}  {}/{}_{}.bin",
        tmp_path,
        bin_name,
        out_dir,
        bin_name,
        env::consts::ARCH
    );
    cmd.exec(cmd1)?;
    cmd.exec(cmd2)?;

    Ok(())
}
