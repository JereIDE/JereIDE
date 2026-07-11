# Release Notes Format

- Always use `## What's Changed` as the title.
- If it is a patch release(only increment the PATCH in semantic versioning) then do not include headers. instead, directly list the changes.
- List the changes in a bulleted list, but an end mark is not required.
- If it is a minor or major release, include headers:

```
## What's Changed

### Added
- Improvement 1
- Improvement 2

### Improved
- Improvement 3
- Improvement 4

### Fixed
- Bug fix 1
- Bug fix 2

### Breaking Changes
- None
```

- Write "- None" if there isn't applicable entries on any header.
