---
name: ui-polish
description: Suggests 10 very minor UI polish improvements, picks the best one, implements it, verifies it builds, then proceeds to the next until all 10 are done. Does not suggest replacing placeholder text/content.
---

# UI Polish Skill

Use this skill when the user asks you to make UI improvements, polish the UI, or do a UI pass on the project. Follow the process below strictly.

## Process

### Step 1: Survey & Suggest

Study the UI codebase to understand what UI framework and patterns are in use. Load relevant views, panels, components, or screens.

Suggest exactly 10 very minor UI polish improvements. Each suggestion must be:

- **Very minor**: A single, small, low-risk change. Examples of appropriate scope:
  - Adjusting padding/margin by a few pixels
  - Tweaking a border radius
  - Changing a font size or weight
  - Adjusting a color shade or opacity
  - Fixing alignment (centering something that's off)
  - Adding a subtle hover effect (color shift, underline)
  - Tweaking transition duration or easing
  - Adjusting icon size relative to text
  - Changing spacing between elements
  - Adding a very faint background or border to distinguish sections
  - Slightly adjusting shadow depth
  - Tweaking line-height for readability
- **Not a placeholder replacement**: Never suggest replacing placeholder text, dummy data, or "TODO" content.
- **Not a feature addition**: No new functionality, no new components, no new pages.
- **Not a refactor**: No restructuring, no renaming, no extracting components.
- **Not a bug fix**: Only cosmetic polish, not behavioral fixes.
- **Low risk**: Should be trivially revertible if something goes wrong.

For each suggestion, include:
- A short title
- The specific file and line(s) to change
- The current value and the proposed new value
- Why it improves the UI

### Step 2: Rank & Pick

Rank the 10 suggestions by impact-to-effort ratio. Pick the single best one to implement first.

### Step 3: Implement

Make the change. Keep it minimal — only touch the specific lines needed. Do not add comments or unrelated changes.

### Step 4: Verify

Run `cargo build` (or the project's equivalent build command) to confirm it compiles. If the project has a way to preview the UI, either launch it or describe what to look for.

If the build fails, fix the issue and rebuild. If you cannot fix it, revert the change and move to the next suggestion.

Then, stage and commit the changes with a simple, non-ai-sh and non-technical commit message. Do not push.

### Step 5: Repeat

Once the change is verified, mark it as done. Pick the next best suggestion from the remaining list and repeat Steps 3-5.

Continue until all 10 improvements have been attempted. Report which ones succeeded, which failed (and why), and a summary of the changes made.
