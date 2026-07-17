# Release Notes Format

- Always use `## What's Changed` as the title. GitHub Actions includes this by default, so no need to include it.
- Always include a title `[RELEASE vVERSION#TITLE]` and a new line, and then start writing the release notes.
- If it is a patch release(only increment the PATCH in semantic versioning) then do not include headers. instead, directly list the changes.
- List the changes in a bulleted list, but an end mark is not required.
- If it is a minor or major release, include headers:

```
[RELEASE vVERSION#TITLE]

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
