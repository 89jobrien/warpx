# Common development commands for WarpX.

default:
    @just --list

# Build and run WarpX locally.
run *args:
    cargo xtask run {{args}}

# Build and install WarpX.app into /Applications.
install *args:
    cargo xtask install {{args}}

# Run WarpX against a local warp-server.
run-local-server port="8080" *args:
    SERVER_ROOT_URL=http://localhost:{{port}} WS_SERVER_URL=ws://localhost:{{port}}/graphql/v2 ./script/run --features with_local_server {{args}}

# Format Rust code.
fmt:
    cargo fmt

# Check Rust formatting.
fmt-check:
    cargo fmt -- --check

# Run the standard workspace tests.
test:
    cargo nextest run --no-fail-fast --workspace --exclude command-signatures-v2

# Run warp_completer tests with v2 features.
test-completer:
    cargo nextest run -p warp_completer --features v2

# Run Rust doc tests.
test-doc:
    cargo test --doc

# Run clippy with warnings denied.
clippy:
    cargo clippy --workspace --all-targets --all-features --tests -- -D warnings

# Run the repository presubmit checks.
presubmit:
    ./script/presubmit

# Format Objective-C/C/C++ sources.
clang-format:
    ./script/run-clang-format.py -r --extensions 'c,h,cpp,m' ./crates/warpui/src/ ./app/src/

# Check WGSL shader formatting.
wgslfmt-check:
    find . -name "*.wgsl" -exec wgslfmt --check {} +

# Install platform-specific dependencies.
bootstrap:
    ./script/bootstrap
