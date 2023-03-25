#!/bin/bash

LOG_FILE="/tmp/sanedit.log"
UCD_VERSION=15.0.0
UCD_DIR="/tmp/ucd-$UCD_VERSION"

function usage {
    echo "Available commands:"
    echo "    release      - create a release build"
    echo "    run          - run in release mode"
    echo "    format       - run formatter"
    echo "    test         - run all tests"
    echo "    debug        - run in debug mode"
    echo "    tail-log     - tail logs ($LOG_FILE) when running"
    echo "    ucd-generate - generate ucd tables v$UCD_VERSION in ucd/src"
    echo "    clean        - clean working directories and builds"
}

function clean {
    rm -rf "$UCD_DIR"
    rm -rf $LOG_FILE
    (cd crates && cargo clean)
}

function release {
    (cd crates && cargo build --release)
}

function run {
    release
    (cd crates && cargo run --release)
}

function run-test {
    (cd crates && cargo test)
}

function debug {
    (cd crates && RUST_BACKTRACE=1 cargo run)
}

function tail-log {
    touch $LOG_FILE
    tail -f $LOG_FILE
}

function download-ucd {
    if test ! -d $UCD_DIR; then
        mkdir -p $UCD_DIR
        cd $UCD_DIR
        curl -LO "https://www.unicode.org/Public/zipped/$UCD_VERSION/UCD.zip"
        unzip UCD.zip
        cd -
    fi
}

function run-ucd-generate {
    download-ucd
    ucd-generate word-break "$UCD_DIR" --enum > crates/ucd/src/word_break.rs
    ucd-generate sentence-break "$UCD_DIR" --enum > crates/ucd/src/sentence_break.rs
    ucd-generate grapheme-cluster-break "$UCD_DIR" --enum > crates/ucd/src/grapheme_break.rs
    ucd-generate general-category "$UCD_DIR" --enum > crates/ucd/src/general_category.rs
    ucd-generate property-bool "$UCD_DIR" --include Extended_Pictographic > crates/ucd/src/properties.rs
    run-format
}

function run-format {
    (cd crates && cargo fmt)
}


case "$1" in
    "release")
        release
        ;;
    "run")
        run
        ;;
    "test")
        run-test
        ;;
    "debug")
        debug
        ;;
    "tail-log")
        tail-log
        ;;
    "ucd-generate")
        run-ucd-generate
        ;;
    "format")
        run-format
        ;;
    "clean")
        clean
        ;;
    *)
    usage
    ;;
esac
