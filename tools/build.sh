#!/bin/bash

set -e

if [[ $TRAVIS_PULL_REQUEST != "false" || -z $TRAVIS_TAG  ]]; then
    cargo build --all
    cargo test --all
    exit 0
fi

version=$TRAVIS_TAG

if [[ $version =~ ^([0-9]+)\.([0-9]+)\.[0-9]+$ ]]; then
    maj="${BASH_REMATCH[1]}"
    min="${BASH_REMATCH[2]}"
else
    exit "bad version: $version"
    exit 1
fi

arch=$(uname -m)

if [[ -z $arch ]]; then
    echo "could not determine arch (uname -m)"
    exit 1
fi

package=setmod-$TRAVIS_OS_NAME-$arch-$version
dest=setmod-$maj.$min

cargo build --release --bin setmod-bot
mkdir $dest

case $TRAVIS_OS_NAME in
"linux")
    cp target/release/setmod-bot $dest/
    cp log4rs.yaml $dest/
    cp secrets.yml.example $dest/
    cp config.toml.example $dest/
    cp tools/setmod-dist.ps1 $dest/setmod.ps1
    zip -r $package.zip $dest/
    ;;
*)
    echo "Unsupported OS: $TRAVIS_OS_NAME"
    exit 1
    ;;
esac

mkdir -p target/upload
cp $package.zip target/upload/
exit 0