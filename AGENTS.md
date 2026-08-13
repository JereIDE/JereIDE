Instead of looking at the egui source code, look at target/doc/egui, where the docs are.  
NEVER INSERT COMMENTS OF YOUR OWN. NEVER.  
Always use `cargo build` instead of `cargo check`.  
Instead of using a subagent to look at the codebase, explore the code directly.
Use the gh cli, and do not use your MCP tools for GitHub.

Note that in this project, the widgets crate is for new generic types with an exposed API that can be used by many different parts of the editor. For example, the palette, the widget palette.

NEVER UPDATE THE DOCS UNLESS YOU ARE EXPLICTLY TOLD TO. But if you updated something that makes the code contradict the docs, remind the user.

For inspection, you can use your MCP egui_mcp to inspect the current running app(you have to ask the user to run it with inspection flags first) and you can click, scroll, hover, etc. You can also save screenshots. If you are a non-vision model, do not open the screenshots you save, but just keep them. If you are a vision model, look at the screenshots.
The app starts by default with the welcome screen; press Cmd+N to open a new tab.
