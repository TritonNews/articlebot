release:
	cargo build --release
	RUST_LOG=info nohup cargo run &

test:
	RUST_LOG=debug cargo run