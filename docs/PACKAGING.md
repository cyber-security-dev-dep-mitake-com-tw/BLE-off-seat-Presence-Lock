# Packaging

## Local (macOS)

```bash
# Universal .app + .dmg + .pkg → dist/macos/
./packaging/macos/build.sh
# or pin a version:
./packaging/macos/build.sh 0.1.0
```

Requires `rustup` targets `aarch64-apple-darwin` and `x86_64-apple-darwin`.

## Local (Windows)

```powershell
cargo build --release
cargo install cargo-wix --locked --version 0.3.9
# WiX Toolset v3 must be on PATH (e.g. choco install wixtoolset)
cargo wix --nocapture
# MSI → target\wix\*.msi
```

Portable EXE is simply `target\release\ble-presence-lock.exe`.

## GitHub Actions

| Workflow | Trigger | Output |
|----------|---------|--------|
| `ci.yml` | push / PR | build + unit tests + Robot tests |
| `release.yml` | tag `v*` or manual dispatch | DMG, PKG, EXE, ZIP, MSI |

Create a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Artifacts are uploaded to the GitHub Release (unsigned). Code signing can be added later via repository secrets.
