use std::process::Command;

use test_log::test;

#[test]
#[ignore]
fn default_config_compiles() -> anyhow::Result<()> {
    let config_dir = tempfile::tempdir()?;

    pinnacle::config::generate_config(config_dir.path(), pinnacle::config::Lang::Rust)?;

    let status = Command::new("cargo")
        .arg("build")
        .current_dir(config_dir.path())
        .spawn()?
        .wait()?;
    assert!(status.success());

    Ok(())
}
