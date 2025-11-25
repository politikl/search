# Search - Terminal Web Browser

**A full terminal-based web browser with vim-style navigation.** Search the web and view pages without ever leaving your terminal.

## Why Search?

### The Problem with Modern Browsing

Every time you open a web browser to look something up, you're subjected to:

- **Tracking cookies** following you across the internet
- **Targeted advertisements** based on your search history
- **Bloated interfaces** with dozens of tabs consuming gigabytes of RAM
- **Distractions** pulling you away from what you actually wanted to find
- **Context switching** that breaks your flow when you're deep in terminal work

As developers, sysadmins, and power users, we spend most of our time in the terminal. Why should a simple search require launching a separate application, waiting for it to load, navigating through a cluttered UI, and then switching back to our work?

### The Solution

Search brings web browsing directly into your terminal with a clean, keyboard-driven interface inspired by vim. No mouse needed. No distractions. No tracking. Just you, your query, and the information you need.

## Features

### Vim-Style Navigation
If you know vim, you already know how to use Search. Press `i` to enter insert mode, navigate with `hjkl`, and `Esc` to return to normal mode. The learning curve is zero for anyone familiar with modal editing.

### In-Terminal Web Rendering
Don't just see search results - actually read the web pages. Search converts HTML to clean, readable text rendered directly in your terminal. Articles, documentation, Stack Overflow answers - all readable without opening a browser.

### Privacy by Design
Search uses Brave Search as its backend, which doesn't track your searches or build advertising profiles. Combined with the fact that you're not loading JavaScript, images, or third-party trackers, your searches remain truly private.

### Lightweight and Fast
Built in Rust for maximum performance. Search launches instantly, fetches results quickly, and uses minimal system resources. No Electron, no WebKit, no bloat.

### Distraction-Free
No ads. No suggested videos. No "people also searched for" rabbit holes. Just the information you asked for, presented cleanly in your terminal.

## Installation

### From Source

```bash
git clone https://github.com/politikl/search.git
cd search
cargo install --path .
```

### Requirements

- Rust toolchain (install from [rustup.rs](https://rustup.rs))
- A terminal emulator with Unicode support

Make sure `~/.cargo/bin` is in your PATH:

```bash
# Add to your .bashrc, .zshrc, or equivalent
export PATH="$HOME/.cargo/bin:$PATH"
```

## Usage

Simply run `search` followed by your query:

```bash
search rust programming
search how to exit vim
search kubernetes pod restart policy
search best practices for API design
```

The TUI will launch showing your search results. Navigate, select, and read - all without leaving your terminal.

### Special Commands

```bash
search about    # Show about information
search history  # View your browsing history
```

The history command shows all pages you've visited along with the search query that led you there and when you visited.

## Keybindings

Search uses a modal interface inspired by vim. There are two main views: Search Results and Web Page, each with Normal and Insert/Browse modes.

### Search Results View

| Mode | Key | Action |
|------|-----|--------|
| NORMAL | `i` | Enter INSERT mode to navigate results |
| NORMAL | `q` | Quit the application |
| INSERT | `j` / `↓` | Move selection down |
| INSERT | `k` / `↑` | Move selection up |
| INSERT | `h` / `l` | Navigate through results |
| INSERT | `Enter` | Open selected page in terminal viewer |
| INSERT | `Esc` | Return to NORMAL mode |

### Web Page View

| Mode | Key | Action |
|------|-----|--------|
| NORMAL | `i` | Enter BROWSE mode to scroll |
| NORMAL | `q` / `Esc` | Return to search results |
| BROWSE | `j` / `↓` | Scroll down 1 line |
| BROWSE | `k` / `↑` | Scroll up 1 line |
| BROWSE | `J` (Shift+j) | Scroll down 10 lines |
| BROWSE | `K` (Shift+k) | Scroll up 10 lines |
| BROWSE | `d` | Scroll down 10 lines (half page) |
| BROWSE | `u` | Scroll up 10 lines (half page) |
| BROWSE | `g` | Jump to top of page |
| BROWSE | `G` | Jump to bottom of page |
| BROWSE | `q` | Return to search results |

## How It Works

1. **Search Query**: Your query is sent to Brave Search's web interface
2. **Result Parsing**: The HTML response is parsed to extract titles, URLs, and descriptions
3. **TUI Display**: Results are rendered in a beautiful terminal interface using ratatui
4. **Page Fetching**: When you select a result, the full page HTML is fetched
5. **Text Conversion**: HTML is converted to readable plain text using html2text
6. **Terminal Rendering**: The page content is displayed with scroll support

All of this happens without executing JavaScript, loading tracking pixels, or storing cookies. What you search stays between you and your terminal.

## Use Cases

### For Developers
- Quickly look up documentation without leaving your editor
- Search Stack Overflow while debugging in the terminal
- Check API references without context switching

### For System Administrators
- Look up command syntax and options
- Search for troubleshooting guides while SSHed into a server
- Find configuration examples without a GUI

### For Privacy-Conscious Users
- Search without being tracked or profiled
- No search history stored anywhere
- No personalized results based on past behavior

### For Productivity Enthusiasts
- Stay in your terminal flow state
- Keyboard-driven interface means faster navigation
- No visual distractions or clickbait

## Technical Details

Search is built with a carefully selected stack of Rust crates:

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client for fetching search results and web pages |
| `scraper` | HTML parsing and CSS selector-based extraction |
| `html2text` | Converting web pages to readable terminal text |
| `ratatui` | Terminal UI framework for the interface |
| `crossterm` | Cross-platform terminal manipulation |

The architecture separates concerns cleanly:
- **Search module**: Handles Brave Search queries and result parsing
- **Fetch module**: Retrieves and converts web pages to text
- **UI module**: Manages the TUI state and rendering
- **Input handling**: Processes keyboard events and mode switching

## Limitations

- **No JavaScript**: Pages that require JavaScript to display content won't render properly
- **No Images**: This is a text-based browser - images are not displayed
- **No Forms**: You cannot submit forms or log into websites
- **No CSS Styling**: Pages are rendered as plain text without visual styling

These limitations are by design. Search is meant for quickly finding and reading information, not for interactive web applications.

## Uninstall

```bash
cargo uninstall search
```

Or manually remove the binary:

```bash
rm ~/.cargo/bin/search
```

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## License

MIT License - feel free to use, modify, and distribute.
