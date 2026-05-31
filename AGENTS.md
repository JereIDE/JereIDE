# JereIDE — Rules for AI Agents

This document is intended for AI coding agents working on JereIDE. It defines the project's platform constraints, coding conventions, and architectural guidelines.

## Platform

- **macOS only.** Never write code for Windows or Linux.
- Minimum deployment target: macOS 12.7+
- Python 3.11+

## Code Style

| Rule | Details |
|------|---------|
| **Naming convention** | Use `camelCase` for variables, functions, and methods (`currentEditor`, `onTabChanged`) |
| **Class names** | `PascalCase` (`MainWindow`, `JereIDEBook`) |
| **Variable names** | Be specific and descriptive — avoid short ambiguous names |
| **Imports** | Prefer explicit imports over `from X import *` |
| **Comments** | Only add comments that explain non-obvious intent or tradeoffs |

## Technology Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| UI framework | PySide6 (Qt6) | Cross-platform Qt bindings |
| Native macOS bridge | PyObjC | For native toolbar, SF Symbols, window styling |
| Terminal emulation | pyte | ANSI terminal emulator for the integrated terminal |

## Architecture Notes

- The entry point is `src/JereIDE.py`.
- All UI components live under `src/ui/`.
- Utility/helper modules live under `src/utils/`.
- Constants and theme values live in `src/const/`.
- The main window (`MainWindow`) owns a `SlidingPanel` that switches between `CodeView` and `CommandView`.
- `CodeView` manages a notebook (`JereIDEBook`) of editor tabs (`QCodeEditor`).
- Editor settings are persisted via `QSettings("Jeremy", "JereIDE")`.

## Do Not

- Do not add dependencies unless they are necessary and approved.
- Do not rewrite or rename files, variables, or functions outside the scope of the task.
- Do not remove existing functionality — focus on additions and targeted fixes.
- Do not add comments that merely restate what the code already says.
- Do not generate code for other platforms.
