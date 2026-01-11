# just manual: https://github.com/casey/just#readme

_default:
	just --list

# Upgrade dependencies, run tests, revert if tests fail
upgrade-deps:
    cargo upgrade -i allow --rust-version `rustc --version | cut -d " " -f2` -vvv && cargo update
    cargo test || (echo "Tests failed, reverting..." && git checkout -- Cargo.toml Cargo.lock && exit 1)

# Setup git hooks (run once after clone)
setup-hooks:
    git config core.hooksPath .githooks

