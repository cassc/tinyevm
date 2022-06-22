prepare:
	cargo install cargo-criterion
build:
	cargo build --release 
test: 
	cargo test 
bench:
	cargo bench
