#!/bin/sh

TESTEXITCODE=$(cargo test --quiet 1>&2)$?
if [ "$TESTEXITCODE" != 0 ]; then
    ehco "Test has failed! You shouldn't push a new version when the tests don't even run in your dev environment!"
    exit "$TESTEXITCODE"
fi

if [ "$1" = "" ]; then
    cargo set-version --bump patch --workspace
    VERSION=$(rg -e "^version\s*=\s*\"(.+)\"" -or '$1' ./**/Cargo.toml --no-filename --no-line-number | head -n1)
else
    VERSION=$1
    cargo set-version "$VERSION" --workspace
fi

echo "Are you sure you want to commit these version changes?"
read -r
git add ./**/Cargo.toml
git commit -m "Bump version to $VERSION"
git tag "v$VERSION"

echo "Are you sure you want to push? (press Enter to push)"
read -r
git push && git push --tags