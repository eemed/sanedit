LOG_FILE="/tmp/eemedit.log"

run:
	RUST_BACKTRACE=1 cargo run

test:
	cargo test

log:
	touch $(LOG_FILE)
	tail -f $(LOG_FILE)


bench-buffer:
	cd eemebuffer; cargo flamegraph --bench some_benchmark --features some_features -- --bench
