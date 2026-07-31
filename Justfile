default:
    @just --list

# Refuse to release from any branch other than main.
_assert-main:
    #!/usr/bin/env bash
    set -euo pipefail
    branch=$(git branch --show-current)
    if [ "$branch" != "main" ]; then
        echo "ERROR: releases can only be run from main (currently on '$branch')" >&2
        exit 1
    fi

# Bump the version, tag and push (see cog.toml); this crate is not packaged.
release type='auto': _assert-main
    cog bump --{{ type }}
