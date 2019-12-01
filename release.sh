#!/bin/bash

set -e

args=("$@")
VERSION=${args[0]}
[ -z "$VERSION" ] && echo "Version not provided" && exit 1

echo "Update version on relevant files"
sed -i '' "s/^version =.*/version = \"$VERSION\"/" Cargo.toml
sed -i '' "s/^version:.*/version: \"$VERSION\"/" joat.yml

echo "Manually update CHANGELOG"
read -sn 1 -p "Press any key to continue..." && echo ""
nvim CHANGELOG.md

echo "Building release version"
cargo build --release

echo "Adding files to the release commit"
read -sn 1 -p "Press any key to continue..." && echo ""
git add -p
echo "Commiting version change"
git commit -m "Version $VERSION"

echo "Tagging last commit with version"
read -sn 1 -p "Press any key to continue..." && echo ""
git tag "$VERSION"

echo "Upload git changes to remote"
git push

echo "Publishing crate"
read -sn 1 -p "Press any key to continue..." && echo ""
cargo publish

echo "Version $VERSION published"
