# Syntax Highlighting

JereIDE colorizes code automatically as you type. Highlighting for the correct language is done based on file extension.

> [!WARNING]
> Syntax highlighting will increase typing latency dramatically. This is under inspection and a fix is coming on the way.

## Language Support

Syntax definitions are stored as JSON files in the app's `data` directory. The following languages are built-in:

- Rust
- Python
- HTML
- Markdown

Other languages are coming, but if you really need support for one look below.

## Custom Language Support

You could make AI generate a highlighting JSON file based on the four already implemented.

> [!IMPORTANT]
> Because some languages have complex syntax and is hard to highlight by plain regex(for example, Markdown and HTML), it would take some code editing to implement for these languages. Make sure your language is completely highlightable by plain regex.

Modify the app's `data` directory. Create a new JSON file and define the highlighting for your definition. Then you edit `languages.json` to point to the new file.
