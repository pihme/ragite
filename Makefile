.PHONY: test
test:
	env -u RAGITE_MODEL_PATH -u RAGITE_MODEL_NAME -u RAGITE_MODEL_REVISION -u RAGITE_QUERY_PREFIX -u RAGITE_PASSAGE_PREFIX \
	cargo build -p ragite-sqlite
	env -u RAGITE_MODEL_PATH -u RAGITE_MODEL_NAME -u RAGITE_MODEL_REVISION -u RAGITE_QUERY_PREFIX -u RAGITE_PASSAGE_PREFIX \
	cargo test --workspace
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
