# Find and Replace

Press `Command/Ctrl+F` or select Find and Replace from the edit menu to open the find/replace palette. It shows as a floating palette, but don't worry, the results will always be visible.

Find and Replace is accessible from the command palette as `editor: find replace`.

If you have text selected in the editor when you trigger the palette, that selection is pre-filled into the Find field.

## Searching

Type into the **Find** field to search the current document. The results will be highlighted and will automatically scroll into view(without getting blocked by the palette). The current result will have a stronger highlight. Cycle through results using `Enter` or selecting the next/previous arrows. The current result vs. all the results will be shown too, as `Current Result Index/Total Results`, e.g. `3/7`.

Everything updates live as you type.

## Options

- **Match case** – only match occurrences with the same casing.
- **Whole word** – only match whole words, not parts of longer words.

## Replacing

Type something into the **Replace with** field.

- **Replace** – replaces the currently selected match and automatically moves to the next result. Pressing `Enter` does the same.
- **Replace All** – replaces every occurrence in the document at once.

Both actions are recorded in the editor's undo history, so you can undo them like any other edit.

## Future

Global project search is coming later.
