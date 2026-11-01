test:
    cargo test -p navi-router-core -p navi-router -p navi-macros -p navi-codegen

test-all:
    cargo test --workspace

# Pedantic is enabled repo-wide via `[workspace.lints]`; `-D warnings` makes
# every warning fatal. The optional features are mutually exclusive blanket
# impls / feature-gated code, so each configuration is linted separately.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings
    for f in validator garde validify valico nested-search; do \
        cargo clippy -p navi-router-core --features $f --all-targets -- -D warnings; \
        cargo clippy -p navi-router --features $f --all-targets -- -D warnings; \
    done

fmt:
    cargo fmt --all -- --check

fmt-fix:
    cargo fmt --all

build:
    cargo build --workspace

coverage:
    cargo llvm-cov --workspace --summary-only

coverage-html:
    cargo llvm-cov --workspace --html --output-dir target/coverage

changelog:
    git cliff --unreleased

verify: fmt clippy test-all

wr:
    watchexec -w ./wr.sh --clear -r "sh ./wr.sh"
