# LSP Support

JereIDE includes builtin LSP (Language Server Protocol) configurations for
common languages. When you open a file, the editor matches its syntax to a
server spec and launches the language server automatically.

## Requirements

The language server binary must be installed and available on your `PATH`. JereIDE
does not install language servers for you.

## Builtin Language Servers

| Language | Server | Binary | Root markers |
|---|---|---|---|
| Rust | rust-analyzer | `rust-analyzer` | `Cargo.toml`, `rust-project.json` |
| C# | OmniSharp | `OmniSharp` | `.sln`, `.csproj` |
| F# | fsautocomplete | `fsautocomplete` | `.fsproj`, `.sln` |
| Java | Eclipse JDT.LS | `jdtls` | `pom.xml`, `build.gradle[.kts]` |
| Kotlin | kotlin-language-server | `kotlin-language-server` | `build.gradle[.kts]`, `pom.xml` |
| Python | Pyright | `pyright-langserver` | `pyproject.toml`, `setup.py`, `pyrightconfig.json` |
| Go | gopls | `gopls` | `go.mod`, `go.work` |
| JavaScript | typescript-language-server | `typescript-language-server` | `tsconfig.json`, `jsconfig.json`, `package.json` |
| TypeScript | typescript-language-server | `typescript-language-server` | `tsconfig.json`, `jsconfig.json`, `package.json` |
| TSX | typescript-language-server | `typescript-language-server` | `tsconfig.json`, `jsconfig.json`, `package.json` |
| PHP | Intelephense | `intelephense` | `composer.json` |
| Elixir | elixir-ls | `elixir-ls` | `mix.exs` |
| OCaml | ocamllsp | `ocamllsp` | `.ocamlformat`, `dune-project`, `*.opam` |
| Gleam | gleam lsp | `gleam lsp` | `gleam.toml` |
| Erlang | erlang_ls | `erlang_ls` | `rebar.config`, `erlang.mk` |
| C/C++ | clangd | `clangd` | `.clangd`, `compile_commands.json` |
| Haskell | haskell-language-server | `haskell-language-server --lsp` | `hie.yaml`, `*.cabal`, `stack.yaml` |
| Zig | zls | `zls` | `build.zig` |
| Dart | Dart SDK | `dart language-server` | `pubspec.yaml` |
| Scala | Metals | `metals` | `build.sbt` |
| Swift | SourceKit-LSP | `sourcekit-lsp` | `Package.swift` |
| Ruby | ruby-lsp | `ruby-lsp` | `Gemfile` |
| Julia | LanguageServer.jl | `julia -e 'using LanguageServer; runserver()'` | `Project.toml` |
| Clojure | clojure-lsp | `clojure-lsp` | `deps.edn`, `project.clj` |
| Crystal | Crystalline | `crystalline` | `shard.yml` |
| Lua | lua-language-server | `lua-language-server` | `.luarc.json` |
| Bash | bash-language-server | `bash-language-server start` | `.git` |
| Gossamer | gossamer-lsp (via `gos`) | `gos lsp` | `project.toml` |

All builtin specs fall back to `.git` as a final root marker.

## Custom Configuration

Create an `lsp.json` file to add servers or override builtins:

- **User-wide:** `~/.config/jereide/lsp.json`
- **Project-specific:** `<project-root>/lsp.json`

Project settings merge on top of user settings, which merge on top of builtins.

### Format

```json
{
  "server_name": {
    "command": ["binary", "--arg"],
    "filetypes": ["language"],
    "rootPatterns": ["marker_file"],
    "initializationOptions": {},
    "settings": {},
    "env": {},
    "autostart": true
  }
}
```

**Fields:**

- `command` (required) -- string or array of strings
- `filetypes` (required) -- array of lowercase language names matching syntax
  file names (e.g. `"rust"`, `"c#"`, `"f#"`, `"javascript"`)
- `rootPatterns` -- files/directories that identify the project root
- `initializationOptions` -- passed to the server on initialize
- `settings` -- server-specific configuration
- `env` -- environment variables for the server process
- `autostart` -- set to `false` to disable a builtin spec

### Examples

Replace pyright with pylsp:

```json
{
  "pyright": { "command": ["echo"], "filetypes": ["_"], "autostart": false },
  "pylsp": {
    "command": ["pylsp"],
    "filetypes": ["python"],
    "rootPatterns": ["pyproject.toml", "setup.py"]
  }
}
```

Add Scala Metals:

```json
{
  "metals": {
    "command": ["metals"],
    "filetypes": ["scala"],
    "rootPatterns": ["build.sbt", "build.sc"]
  }
}
```
