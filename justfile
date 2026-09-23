# justfile pb-covers

# If invoked in CI. Either 'true' or 'false'
ci := "false"
# Cargo build profile
cargo_profile := "dev"
# Pocketbook device identifier as it's folder name when connected with USB.
# - Pocketbook Inkpad 4: "PB743G"
# - Pocketbook Touch Lux 3: "PB626"
pb_device := "PB626"
# Pocketbook libc version
# - Pocketbook Inkpad 4: "2.41"
# - Pocketbook Touch Lux 3: "2.23"
pb_libc_version := "2.23"
# Pocketbook SDK version
# - Pocketbook Inkpad 4: "6.11"
# - Pocketbook Touch Lux 3: "5.19"
pb_sdk_version := "5.19"
# Build target triple for Pocketbook device
pb_build_target := "armv7-unknown-linux-gnueabi"
# Pocketbook SSH host
pb_ssh_host := "pb-touchlux3-dropbear"

[private]
sudo_cmd := if ci == "true" { "" } else { "sudo" }
[private]
linux_distr := `grep -o -E '^ID=([a-zA-Z0-9_]*)$' -r /etc/os-release | cut -d= -f2 | tr '[:upper:]' '[:lower:]'`
[private]
cargo_build_target := pb_build_target
[private]
cargo_zigbuild_target := cargo_build_target + "." + pb_libc_version
[private]
cargo_out_profile := if cargo_profile == "dev" { "debug" } else { cargo_profile }
[private]
cargo_sdk_feature := "sdk-" + replace(pb_sdk_version, ".", "-")

export RUST_LOG := env("RUST_LOG", "pb_covers=debug,pb_covers_cli=debug,pb_covers_simulator=debug")
export RUST_BACKTRACE := env("RUST_BACKTRACE", "1")

default:
    just --list

[confirm]
clean:
    cargo clean

# Perform prerequisite steps like installing dependencies, tools and toolchains.
prerequisites:
    #!/usr/bin/env bash
    set -euxo pipefail
    if [[ ('{{linux_distr}}' =~ 'fedora') ]]; then
        {{sudo_cmd}} dnf install -y zig execstack SDL2-devel
    elif [[ '{{linux_distr}}' =~ 'debian' || '{{linux_distr}}' =~ 'ubuntu' ]]; then
        {{sudo_cmd}} apt-get update
        {{sudo_cmd}} apt-get install -y zig execstack sdl2-dev
    else
        echo "Can't install system dependencies, unsupported distro."
        exit 1
    fi
    rustup target add {{cargo_build_target}}
    cargo install --locked cargo-zigbuild

# Build the app.
build:
    cargo zigbuild \
        --target {{cargo_zigbuild_target}} \
        --profile {{cargo_profile}} \
        --features="inkview {{cargo_sdk_feature}}" \
        --bin pb-covers
    execstack -s {{ "target" / cargo_build_target / cargo_out_profile / "pb-covers" }}

build-cli:
    cargo build \
        --profile {{cargo_profile}} \
        --bin pb-covers-cli

# Format the code.
fmt:
    cargo fmt

# Lint the app.
lint:
    cargo clippy \
        --features="inkview {{cargo_sdk_feature}} simulator"

# Run the cli.
run-cli *ARGS:
    cargo run \
        --profile {{cargo_profile}} \
        --features="inkview {{cargo_sdk_feature}}" \
        --bin pb-covers-cli \
        {{ARGS}}

# Run the cli.
run-simulator *ARGS:
    cargo run \
        --profile {{cargo_profile}} \
        --features="simulator" \
        --bin pb-covers-simulator \
        {{ARGS}}

# Deploy the application to the device over USB.
deploy-usb: build
    cp {{ "target" / cargo_build_target / cargo_out_profile / "pb-covers" }} \
        {{"/run/media/$USER" / pb_device / "applications" / "pb-covers.app" }}
    sync

[doc('Deploy the application to the device over SSH.
Make sure a SSH connection is available.')]
deploy-ssh: build
    scp {{ "target" / cargo_build_target / cargo_out_profile / "pb-covers"}} \
        {{pb_ssh_host}}:/mnt/ext1/applications/pb-covers.app

[doc('Launch a GDB server session on the device.
Make sure a SSH connection is available.')]
launch-gdbserver:
    ssh {{pb_ssh_host}} gdbserver 0.0.0.0:10003 /mnt/ext1/applications/pb-covers.app 
