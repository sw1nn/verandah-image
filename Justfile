default:
    @just --list

# Bump the version, tag and push (see cog.toml); this crate is not packaged.
release type='auto':
    cog bump --{{ type }}
