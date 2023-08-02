use std::process::Command;
use std::io::Result;

pub fn compile_protos(_protos: &[&str], _includes: &[&str]) -> Result<()> {
    let mut cmd = Command::new("protoc");
    cmd.arg("--include_imports")
        .arg("--include_source_info")
        .arg("-o")
        .arg("gtfs-realtime.pb");

    Ok(())
}
