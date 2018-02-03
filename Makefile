release:
	cargo build --release
	RUST_LOG=info nohup cargo run &