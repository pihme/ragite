.PHONY: test
test:
	cargo build -p ragite-sqlite
	cargo test --workspace
