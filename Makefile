.PHONY: test
test:
	cargo build -p ragite-sqlite
	cargo test --workspace
	RAGITE_EXTENSION="$(CURDIR)/target/debug/libragite.so" python3 tests/languages/python/load_test.py
	RAGITE_EXTENSION="$(CURDIR)/target/debug/libragite.so" node --test tests/languages/node/load_test.mjs
	cd tests/languages/go && RAGITE_EXTENSION="$(CURDIR)/target/debug/libragite.so" GOTOOLCHAIN=local GOPROXY=off GOSUMDB=off CGO_ENABLED=1 go test -count=1 .
