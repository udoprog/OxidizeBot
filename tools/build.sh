#!/bin/bash

set -e

case $TRAVIS_OS_NAME in
"windows")
    export SQLITE3_LIB_DIR="${PWD}/bot/lib"
    choco install 7zip
    ;;
"linux")
    ;;
*)
    echo "Unsupported OS: $TRAVIS_OS_NAME"
    exit 1
    ;;
esac

if [[ $TRAVIS_PULL_REQUEST != "false" ]]; then
    cargo build --all
    cargo test --all
    exit 0
fi

cargo build --release --all
cargo test --release --all

dest=setmod-$TRAVIS_COMMIT

mkdir $dest

# example secrets.yml
cp secrets.yml.example $dest/

case $TRAVIS_OS_NAME in
"windows")
    cp target/release/setmod-bot.exe $dest/
    7z a $dest.zip $dest/
    ;;
"linux")
    cp target/release/setmod-bot $dest/
    zip -r $dest.zip $dest/
    ;;
*)
    echo "Unsupported OS: $TRAVIS_OS_NAME"
    exit 1
    ;;
esac

mkdir -p target/upload
cp $dest.zip target/upload/
exit 0