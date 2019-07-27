#!/bin/bash

set -e

if [[ $TRAVIS_PULL_REQUEST != "false" || -z $TRAVIS_TAG  ]]; then
    cargo test --manifest-path=bot/Cargo.toml --no-default-features
    exit 0
fi

version=$TRAVIS_TAG

if ! [[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-.+)?$ ]]; then
    exit "bad version: $version"
    exit 1
fi

arch=$(uname -m)

if [[ -z $arch ]]; then
    echo "could not determine arch (uname -m)"
    exit 1
fi

zip=$PWD/oxidize-$version-$TRAVIS_OS_NAME-$arch.zip

cargo build --release --bin oxidize
mkdir target/build

case $TRAVIS_OS_NAME in
"linux")
    cp README.md target/build
    cp target/release/oxidize target/build
    (cd target/build; zip -r $zip *)
    ;;
*)
    echo "Unsupported OS: $TRAVIS_OS_NAME"
    exit 1
    ;;
esac

mkdir -p target/upload
cp $zip target/upload/

(cd bot && cargo deb)
cp target/debian/oxidize_*_amd64.deb target/upload/
exit 0