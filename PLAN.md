# Rebranding Plan: Lite Anvil → JereIDE

## Decisions

| Item | Decision |
|------|----------|
| Display name | `JereIDE` (no space) |
| Binary name | `jereide` (no hyphen) |
| Config dirs | `jereide` |
| macOS bundle ID | `com.jeremy.jereide` |
| Linux app IDs | `com.jeremy.jereide` |
| Changelog | Delete entirely |
| Update check URL | Remove |
| Icons | Placeholder |
| Generated docs/ | Leave as-is |
| .gitignore | Keep as-is |
| LICENSE | Append `Jeremy-Qian` to line 4 |
| Sub-editors (nano/note) | Remove entirely |
| Package description | `JereIDE - a fast code editor built in Rust with SDL3` |
| Cargo.toml authors | Replace with `Jeremy-Qian` |

## Phase 1: Delete nano-anvil and note-anvil crates

- [x] 1. Delete directories: `nano-anvil/`, `note-anvil/`
- [x] 2. Delete files: `resources/linux/com.nano_anvil.NanoAnvil.desktop`, `resources/linux/com.note_anvil.NoteAnvil.desktop`
- [x] 3. Delete: `resources/icons/nano-anvil.png`, `resources/icons/note-anvil.png`
- [x] 4. Delete: `resources/linux/com.lite_anvil.LiteAnvil.metainfo.xml.in` (will recreate with new name)
- [ ] 5. Remove remaining nano/note/lite-anvil references from:
      - `scripts/install-mac.sh`
      - `scripts/build-sdl3-nogl.sh`
      - `install.sh` (lite-anvil.png cleanup)
      - `scripts/innosetup/innosetup.iss.in`

## Phase 2: Rename crate directories and Cargo.toml

- [x] 6. Rename `anvil-core/` → `jereide-core/`
- [x] 7. Rename `lite-anvil/` → `jereide/`
- [x] 8. Update root `Cargo.toml`: `members = ["jereide-core", "jereide"]`
- [x] 9. Update `jereide-core/Cargo.toml`
- [x] 10. Update `jereide/Cargo.toml`

## Phase 3: Core source code strings

- [x] 11. `jereide-core/src/window.rs:114` — `"Lite Anvil"` → `"JereIDE"`
- [x] 12. `jereide-core/src/window.rs:118` — `"lite-anvil"` → `"jereide"`
- [x] 13. `jereide-core/src/window.rs:107-110` — Update comments
- [x] 14. `jereide-core/src/window.rs:227` — icon path
- [x] 15. `jereide-core/src/window.rs:222-224` — Update comments
- [x] 16. `jereide-core/src/runtime.rs:87-91` — strings + remove nano/note arms
- [x] 17. `jereide-core/src/runtime.rs:100` — `"lite-anvil"` → `"jereide"`
- [x] 18. `jereide-core/src/runtime.rs:112` — Comment update
- [x] 19. `jereide-core/src/runtime.rs:147-154` — Comments, nano/note arms, strings

## Phase 4: Editor source strings

- [x] 20. `jereide-core/src/editor/main_loop.rs` — All `"Lite Anvil"` → `"JereIDE"`
- [x] 21. `jereide-core/src/editor/main_loop.rs` — `"lite-anvil.log"` → `"jereide.log"`
- [x] 22. `jereide-core/src/editor/title_view.rs:32` — `"Lite Anvil"` → `"JereIDE"`
- [x] 23. `jereide-core/src/editor/config.rs:356` — config header
- [x] 24. `jereide-core/src/editor/config_template.toml:1` — config header
- [x] 25. `jereide-core/src/editor/commands_dispatch.rs:616` — GitHub releases URL
- [x] 26. `jereide-core/src/editor/subsystems.rs` — comments
- [x] 27. `jereide-core/src/editor/picker.rs:491-492` — test strings

## Phase 5: Platform resources

- [x] 28. Create `resources/linux/com.jeremy.jereide.desktop`
- [ ] 29. Create `resources/linux/com.jeremy.jereide.metainfo.xml.in`
- [x] 30. Update `resources/macos/Info.plist`
- [x] 31. Update `resources/windows/install-file-associations.ps1`
- [x] 32. Update `resources/windows/uninstall-file-associations.ps1`
- [x] 33. Rename icon `lite-anvil.png` → `jereide.png`

## Phase 6: Build & packaging scripts

- [ ] 34. Update `scripts/innosetup/innosetup.iss.in` — all references
- [x] 35. Update `scripts/build-local-linux.sh`
- [x] 36. Update `scripts/build-local-mac.sh`
- [x] 37. Update `scripts/build-local-win.ps1`
- [ ] 38. Update `scripts/install-mac.sh` — all references
- [ ] 39. Update `install.sh` — all references, remove nano/note
- [x] 40. Update `install.ps1`
- [x] 41. Update `uninstall.sh`
- [x] 42. Update `.github/workflows/release.yml`

## Phase 7: Documentation

- [x] 43. Rewrite `README.md`
- [x] 44. Update `BUILDING.md`
- [x] 45. Update `LSP_SUPPORT.md`
- [x] 46. Delete `changelog.md`
- [x] 47. Update `mkdocs.yml`
- [x] 48. Update `docs_src/index.md`
- [x] 49. Update `docs_src/about.md`
- [ ] 50. Update `docs_src/installation.md` — all paths and binary names
- [x] 51. Update `docs_src/guide.md`
- [x] 52. Update `docs_src/screenshots.md`
- [x] 53. Update `docs_src/stylesheets/extra.css:1` — comment

## Phase 8: License & final cleanup

- [ ] 54. Update `LICENSE:4` — `Copyright (c) 2026-present Dan Pozmanter, Jeremy-Qian`
- [x] 55. Update `jereide/src/main.rs:6` — comment
- [ ] 56. Clean up `jereide-core/tests/baseline_perf.rs` — hardcoded developer paths
- [x] 57. Verify: `cargo build` succeeds
- [ ] 58. Verify: `cargo test` passes
- [ ] 59. Verify: `cargo clippy` clean
