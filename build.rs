fn main() {
    vergen::EmitBuilder::builder()
        .cargo_debug()
        .cargo_target_triple()
        .git_sha(true)
        .git_branch()
        .git_commit_message()
        .git_dirty(false)
        .rustc_semver()
        .sysinfo_os_version()
        .emit()
        .unwrap();
}
