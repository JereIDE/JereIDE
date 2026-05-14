<div align="center"><h1>JereIDE</h1></div>

A Pyside6 + PyObjC implementation of [JereIDE_wx](https://github.com/Jeremy-Qian/JereIDE_wx).  Still in beta.  

This project was initially a vibe coding project, but I edit the code manually more and more.

## Installation

### Prerequisites

- macOS 12.7+
- Python 3.11+

### Dependencies

| Package | Purpose |
|---------|---------|
| `PySide6` | Qt6 bindings — core UI framework |
| `pyobjc` | Bridge to native macOS APIs (toolbar, SF Symbols) |
| `pyte` | Terminal emulator for the integrated terminal |

### Setup

```bash
# Clone the repository
git clone https://github.com/Jeremy-Qian/JereIDE.git
cd JereIDE

# Install dependencies
pip install -r requirements.txt

# Run the application
python3 src/JereIDE.py
```

> **Note:** This application is optimized for macOS only.

## Stars
<a href="https://www.star-history.com/?repos=Jeremy-Qian%2FJereIDE&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=Jeremy-Qian/JereIDE&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=Jeremy-Qian/JereIDE&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=Jeremy-Qian/JereIDE&type=date&legend=top-left" />
 </picture>
</a>

## Rules for AI Agents
This app is optimized for MAC only. It is built by mostly AI.

If you are an agent:
Do not write code for other platforms except for macOS. When writing code, use camelCase and use specific variable names.

## Known Limitations
- Find/Replace: "Regex", "Whole Words", and "Wrap Around" options are not yet implemented and are disabled in the UI.

## Plans for the future
- [x] Docstring highlighting
- [ ] Save all, recent files
- [ ] Command View  
- [ ] Find/Replace: regex, whole words, wrap options

For a full list of what's coming next, see the [JereIDE Roadmap](https://github.com/users/Jeremy-Qian/projects/4/views/1).

## License
This project is licensed under the MIT License.
