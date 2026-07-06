# Build the C dependency driven through cmake-rs (SDL3) against the static,
# non-debug MSVC runtime, so every object links the same CRT that rustc does:
# under +crt-static (see ../.cargo/config.toml) the Rust side links the static
# non-debug runtime (libcmt) for every profile, including debug.
#
# SDL's cmake_minimum_required selects policy CMP0091 NEW, where the runtime is
# chosen by this variable rather than by the /MT or /MD compile flag that `cc`
# already passes. The value is deliberately not config-aware: a Debug configure
# must still use /MT (libcmt), never /MTd (libcmtd), to stay on rustc's CRT.
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded")
