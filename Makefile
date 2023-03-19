LOG_FILE="/tmp/eemedit.log"
UCD_VERSION=15.0.0

default:
	@echo "Available commands:"
	@echo "    release      - create a release build"
	@echo "    run          - run in release mode"
	@echo "    test         - run all tests"
	@echo "    debug        - run in debug mode"
	@echo "    tail-log     - tail logs when running"
	@echo "    ucd-generate - regenerate ucd tables in ucd/src"
	@echo ""

debug:
	RUST_BACKTRACE=1 cargo run

release:
	cargo build --release

run:
	make release
	cargo run --release

test:
	cargo test

tail-log:
	touch $(LOG_FILE)
	tail -f $(LOG_FILE)

ucd-generate:
	make download-ucd
	ucd-generate word-break /tmp/ucd-$(UCD_VERSION) --chars > ucd/src/word_break.rs
	ucd-generate sentence-break /tmp/ucd-$(UCD_VERSION) --chars > ucd/src/sentence_break.rs
	ucd-generate grapheme-cluster-break /tmp/ucd-$(UCD_VERSION) --chars > ucd/src/grapheme_break.rs
	ucd-generate general-category /tmp/ucd-$(UCD_VERSION) --chars > ucd/src/general_category.rs
	rm -rf /tmp/ucd-$(UCD_VERSION)

download-ucd:
	rm -rf /tmp/ucd-$(UCD_VERSION)
	mkdir /tmp/ucd-$(UCD_VERSION)
	cd /tmp/ucd-$(UCD_VERSION) && curl -LO https://www.unicode.org/Public/zipped/$(UCD_VERSION)/UCD.zip && unzip UCD.zip
