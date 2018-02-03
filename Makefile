test:
	cargo build
	cargo run

release:
	cargo build --release
	nohup cargo run &

clean:
	cargo clean