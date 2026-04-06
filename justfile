# Build in debug mode
debug:
    cargo build

# Build in release mode
release:
    cargo build --release

# Run the app in debug mode
run:
    cargo run

# Run the app in release mode
run-release:
    cargo run --release

# Run all tests
test:
    cargo test --all-targets

# Run unit tests only
test-unit:
    cargo test --lib

# Run integration tests only
test-integration:
    cargo test --test integration_tests

# Format code
format:
    cargo fmt

# Check formatting without modifying
format-check:
    cargo fmt -- --check

# Run clippy linter
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all checks (format + lint + test)
check: format-check lint test

# Build macOS .app bundle (default: release)
app config="release" version="dev":
    ./scripts/build-app.sh {{ config }} {{ version }}

# Run as macOS .app (builds release first)
run-app: (app "release")
    open build/Gituqueiro.app

# Generate icon files from SVG source
icon:
    ./scripts/build-icons.sh

# Clean build artifacts
clean:
    cargo clean
