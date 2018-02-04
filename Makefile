LOGPATH = logs
LOGFILE = $(LOGPATH)/$(shell date --iso=seconds)

release:
	cargo build --release
	RUST_LOG=info nohup cargo run > $(LOGFILE) &

test:
	RUST_LOG=debug cargo run