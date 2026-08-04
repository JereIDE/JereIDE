Instead of looking at the egui source code, look at target/doc/egui, where the docs are.  
NEVER INSERT COMMENTS OF YOUR OWN. NEVER.  
Always use `cargo build` instead of `cargo check`.  
Instead of using a subagent to look at the codebase, explore the code directly.
Use the gh cli, and do not use your MCP tools.

Note that in this project, the widgets crate is for new generic types with an exposed API that can be used by many different parts of the editor. For example, the palette, the widget palette.

NEVER UPDATE THE DOCS UNLESS YOU ARE EXPLICTLY TOLD TO. But if you updated something that makes the code contradict the docs, remind the user.
