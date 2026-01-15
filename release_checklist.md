# Release checklist

Keeping this here because I'm inevitably going to forget

- [ ] Bump Cargo.toml version
- [ ] Run cargo build so the Cargo.lock updates
- [ ] Bump versions in nix files
- [ ] Copy rockspec to new version
- [ ] Bump versions in new rockspec
    - Ideally fix the CI to not need to do that all the time
- [ ] If changes in Rust API, bump that version
- [ ] If wiki has changes, move to new version
- [ ] Bump and test AUR PKGBUILD
