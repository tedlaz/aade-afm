# Build για Windows 7 / 8 / 8.1 / 10 / 11 (64-bit).
# Απαιτεί: rustup toolchain install nightly --component rust-src
cargo +nightly build --release -Z build-std=std,panic_abort --target x86_64-win7-windows-msvc
if ($LASTEXITCODE -eq 0) { Copy-Item target\x86_64-win7-windows-msvc\release\aade-afm.exe . ; "OK -> aade-afm.exe" }
