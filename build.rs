use vergen_gitcl::{CargoBuilder, Emitter, GitclBuilder, RustcBuilder, SysinfoBuilder};

fn main() {
    let cargo = CargoBuilder::default()
        .debug(true)
        .target_triple(true)
        .build()
        .unwrap();
    let git = GitclBuilder::default()
        .sha(true)
        .branch(true)
        .commit_message(true)
        .dirty(false)
        .build()
        .unwrap();
    let rustc = RustcBuilder::default().semver(true).build().unwrap();
    let sysinfo = SysinfoBuilder::default().os_version(true).build().unwrap();

    Emitter::default()
        .add_instructions(&cargo)
        .unwrap()
        .add_instructions(&git)
        .unwrap()
        .add_instructions(&rustc)
        .unwrap()
        .add_instructions(&sysinfo)
        .unwrap()
        .emit()
        .unwrap();
}
