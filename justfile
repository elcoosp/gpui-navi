test:
    cargo test -p navi-router-core -p navi-router -p navi-macros -p navi-codegen -p example-app

clippy:
    cargo clippy --all-targets --all-features --workspace -- -D warnings

fmt:
    cargo fmt --all -- --check

build:
    cargo build --workspace

wr:
    watchexec -w ./wr.sh --clear -r "sh ./wr.sh"
