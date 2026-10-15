.PHONY: test
test:
	# The license test runs `cargo metadata --offline`, which resolves every
	# target. A host build never downloads wasm-only crates such as bumpalo.
	cargo fetch --locked
	env -u RAGITE_MODEL_PATH -u RAGITE_MODEL_NAME -u RAGITE_MODEL_REVISION -u RAGITE_QUERY_PREFIX -u RAGITE_PASSAGE_PREFIX \
	cargo build -p ragite-sqlite
	env -u RAGITE_MODEL_PATH -u RAGITE_MODEL_NAME -u RAGITE_MODEL_REVISION -u RAGITE_QUERY_PREFIX -u RAGITE_PASSAGE_PREFIX \
	cargo test --workspace
	RAGITE_EXTENSION="$(CURDIR)/target/debug/libragite.so" python3 tests/languages/python/load_test.py
	RAGITE_EXTENSION="$(CURDIR)/target/debug/libragite.so" node --test tests/languages/node/load_test.mjs
	cd tests/languages/go && RAGITE_EXTENSION="$(CURDIR)/target/debug/libragite.so" GOTOOLCHAIN=local GOPROXY=off GOSUMDB=off CGO_ENABLED=1 go test -count=1 .
	env -u RAGITE_MODEL_PATH -u RAGITE_MODEL_NAME -u RAGITE_MODEL_REVISION -u RAGITE_QUERY_PREFIX -u RAGITE_PASSAGE_PREFIX \
	cargo test -p ragite-core --no-default-features --lib
	env -u RAGITE_MODEL_PATH -u RAGITE_MODEL_NAME -u RAGITE_MODEL_REVISION -u RAGITE_QUERY_PREFIX -u RAGITE_PASSAGE_PREFIX \
	cargo build -p ragite-sqlite --no-default-features --target-dir target/nomodel
	env -u RAGITE_MODEL_PATH -u RAGITE_MODEL_NAME -u RAGITE_MODEL_REVISION -u RAGITE_QUERY_PREFIX -u RAGITE_PASSAGE_PREFIX \
	cargo test -p ragite-sqlite --no-default-features --target-dir target/nomodel --test nomodel -- --ignored
	RAGITE_MODEL_PATH='$(CURDIR)/models/potion-base-8M' \
	RAGITE_MODEL_NAME=provider-demo \
	RAGITE_MODEL_REVISION=rev-supplied \
	RAGITE_QUERY_PREFIX='query:' \
	RAGITE_PASSAGE_PREFIX='passage:' \
	cargo build -p ragite-sqlite --no-default-features --target-dir target/provider
	RAGITE_MODEL_PATH='$(CURDIR)/models/potion-base-8M' \
	RAGITE_MODEL_NAME=provider-demo \
	RAGITE_MODEL_REVISION=rev-supplied \
	RAGITE_QUERY_PREFIX='query:' \
	RAGITE_PASSAGE_PREFIX='passage:' \
	cargo test -p ragite-sqlite --no-default-features --target-dir target/provider --test provider -- --ignored
	RAGITE_MODEL_PATH='$(CURDIR)/models/potion-base-8M' \
	RAGITE_MODEL_NAME=provider-demo \
	RAGITE_MODEL_REVISION=rev-supplied \
	RAGITE_QUERY_PREFIX='query:' \
	RAGITE_PASSAGE_PREFIX='passage:' \
	cargo test -p ragite-core --no-default-features --target-dir target/provider --lib tc_54_tc_55_tc_126_provider_prefixes
