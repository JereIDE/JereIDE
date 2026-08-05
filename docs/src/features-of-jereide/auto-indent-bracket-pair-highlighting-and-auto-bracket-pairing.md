# Auto Indent, Bracket Pair Highlighting, and Auto Bracket Pairing

This page covers 3 of the smaller features working behind the scenes.

## Auto Indent

In the future this might smart-indent based on language, but currently it just copies the previous line, whether it is made of spaces or tabs. Simple, and it works.

## Bracket Pair Highlighting

If you are inside a bracket and your cursor is near one of them, both brackets get highlighted. This feature is present in many, many editors and JereIDE would be incomplete without it. It helps you find where your current level is at in a confusing nest of brackets.

## Auto Bracket Pairing

When you type `{`, `(`, `[`, or `<`, the closing bracket(`}`, `)`, `]`, or `>`) is automatically inserted. If you happen to type the closing bracket yourself by habit, it won't turn into `{}}`. Your cursor would just jump to after the closing bracket, as if auto-bracket-pairing had never happened.

If you delete the opening bracket of an empty pair, the closing bracket will be deleted too.

> [!IMPORTANT]
> The current implementations are not smart.
> In the future a better implementation will include the smart features that [Zed](https://zed.dev) somehow achieves.
