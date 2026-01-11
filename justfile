# just manual: https://github.com/casey/just#readme

_default:
	just --list

upgrade-deps:
    cargo upgrade -i allow --rust-version `rustc --version | cut -d " " -f2` -vvv && cargo update

