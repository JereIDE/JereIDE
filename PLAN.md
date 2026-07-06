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

1. Delete directories: `nano-anvil/`, `note-anvil/`
2. Delete files: `resources/linux/com.nano_anvil.NanoAnvil.desktop`, `resources/linux/com.note_anvil.NoteAnvil.desktop`
3. Delete: `resources/icons/nano-anvil.png`, `resources/icons/note-anvil.png`
4. Delete: `resources/linux/com.lite_anvil.LiteAnvil.metainfo.xml.in` (will recreate with new name)
5. Remove all nano/note references from:
   - `Cargo.toml` workspace members
   - `anvil-core/src/runtime.rs` (nano-anvil/note-anvil match arms)
   - `anvil-core/src/window.rs` (nano/note override comments)
   - `install.sh`, `uninstall.sh`, `install.ps1`
   - `scripts/build-local-linux.sh`, `scripts/build-local-mac.sh`, `scripts/build-local-win.ps1`
   - `.github/workflows/release.yml`
   - `README.md`, `BUILDING.md`, `docs_src/*.md`
   - `mkdocs.yml`

## Phase 2: Rename crate directories and Cargo.toml

6. Rename `anvil-core/` → `jereide-core/`
7. Rename `lite-anvil/` → `jereide/`
8. Update root `Cargo.toml`: `members = ["jereide-core", "jereide"]`
9. Update `jereide-core/Cargo.toml`:
   - `name = "jereide-core"`
   - `description = "JereIDE - a fast code editor built in Rust with SDL3"`
   - `authors = ["Jeremy-Qian"]`
10. Update `jereide/Cargo.toml`:
    - `name = "jereide"`
    - `description = "JereIDE - a fast code editor built in Rust with SDL3"`
    - `authors = ["Jeremy-Qian"]`
    - All asset paths: update `anvil-core` → `jereide-core`, icon paths
    - Deb/rpm metadata if present

## Phase 3: Core source code strings

11. `jereide-core/src/window.rs:114` — `"Lite Anvil"` → `"JereIDE"`
12. `jereide-core/src/window.rs:118` — `"lite-anvil"` → `"jereide"`
13. `jereide-core/src/window.rs:107-110` — Update comments
14. `jereide-core/src/window.rs:227` — `include_bytes!("../../resources/icons/lite-anvil.png")` → `include_bytes!("../../resources/icons/jereide.png")`
15. `jereide-core/src/window.rs:222-224` — Update comments
16. `jereide-core/src/runtime.rs:87-91` — All `"lite-anvil"` → `"jereide"`, remove nano/note match arms
17. `jereide-core/src/runtime.rs:100` — `"lite-anvil"` → `"jereide"`
18. `jereide-core/src/runtime.rs:112` — Comment update
19. `jereide-core/src/runtime.rs:147-154` — Update comments, remove nano/note match arms, `"lite-anvil"` → `"jereide"`

## Phase 4: Editor source strings

20. `jereide-core/src/editor/main_loop.rs` — All `"Lite Anvil"` → `"JereIDE"` (lines 67, 642, 767, 786, 850, 4998, 9338, 9463)
21. `jereide-core/src/editor/main_loop.rs:12769` — `"lite-anvil.log"` → `"jereide.log"`
22. `jereide-core/src/editor/title_view.rs:32` — `"Lite Anvil"` → `"JereIDE"`
23. `jereide-core/src/editor/config.rs:356` — `"# Lite Anvil configuration\n\n"` → `"# JereIDE configuration\n\n"`
24. `jereide-core/src/editor/config_template.toml:1` — `# Lite Anvil configuration` → `# JereIDE configuration`
25. `jereide-core/src/editor/commands_dispatch.rs:616` — Remove/update GitHub releases URL
26. `jereide-core/src/editor/subsystems.rs:3,107,143` — Update comments
27. `jereide-core/src/editor/picker.rs:491-492` — `lite-anvil` → `jereide` in test strings

## Phase 5: Platform resources

28. Create `resources/linux/com.jeremy.jereide.desktop` with:
    - `Name=JereIDE`
    - `Exec=jereide`
    - `Icon=com.jeremy.jereide`
    - `StartupWMClass=jereide`
29. Create `resources/linux/com.jeremy.jereide.metainfo.xml.in` with:
    - `<id>com.jeremy.jereide</id>`
    - `<name>JereIDE</name>`
    - `<binary>jereide</binary>`
30. Update `resources/macos/Info.plist`:
    - `CFBundleName: JereIDE`
    - `CFBundleIdentifier: com.jeremy.jereide`
    - `CFBundleExecutable: jereide`
    - `CFBundleIconFile: jereide.icns`
31. Update `resources/windows/install-file-associations.ps1` — all references
32. Update `resources/windows/uninstall-file-associations.ps1` — all references
33. Rename `resources/icons/lite-anvil.png` → `resources/icons/jereide.png` (placeholder)

## Phase 6: Build & packaging scripts

34. Update `scripts/innosetup/innosetup.iss.in` — all references
35. Update `scripts/build-local-linux.sh` — archive names, binary names
36. Update `scripts/build-local-mac.sh` — `LiteAnvil.app` → `JereIDE.app`, all names
37. Update `scripts/build-local-win.ps1` — archive names, binary name
38. Update `scripts/install-mac.sh` — all references
39. Update `install.sh` — all references, remove nano/note
40. Update `install.ps1` — all references, remove nano/note
41. Update `uninstall.sh` — all references, remove nano/note
42. Update `.github/workflows/release.yml` — all binary names, archive names, artifact names

## Phase 7: Documentation

43. Rewrite `README.md` — title, all references, remove nano/note sections, remove GitHub URLs (keep local)
44. Update `BUILDING.md` — all `lite-anvil` → `jereide`, remove nano/note references
45. Update `LSP_SUPPORT.md` — `~/.config/lite-anvil/lsp.json` → `~/.config/jereide/lsp.json`
46. Delete `changelog.md`
47. Update `mkdocs.yml` — `site_name: JereIDE`, remove `site_url`, `repo_url`, `repo_name`
48. Update `docs_src/index.md` — all references, remove nano/note
49. Update `docs_src/about.md` — all references
50. Update `docs_src/installation.md` — all paths and binary names
51. Update `docs_src/guide.md` — all references
52. Update `docs_src/screenshots.md` — titles and image references
53. Update `docs_src/stylesheets/extra.css:1` — comment

## Phase 8: License & final cleanup

54. Update `LICENSE:4` — `Copyright (c) 2026-present Dan Pozmanter, Jeremy-Qian`
55. Update `jereide/src/main.rs:6` — comment
56. Clean up `jereide-core/tests/baseline_perf.rs` — hardcoded developer paths
57. Verify: `cargo build` succeeds
58. Verify: `cargo test` passes
59. Verify: `cargo clippy` clean
