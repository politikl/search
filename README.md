# Navim - Terminal Web Browser

**A full terminal-based web browser with vim-style navigation.** Search the web and view pages without ever leaving your terminal.

## Why Navim?

### The Problem with Modern Browsing

Every time you open a web browser to look something up, you're subjected to:

- **Tracking cookies** following you across the internet
- **Targeted advertisements** based on your search history
- **Bloated interfaces** with dozens of tabs consuming gigabytes of RAM
- **Distractions** pulling you away from what you actually wanted to find
- **Context switching** that breaks your flow when you're deep in terminal work

As developers, sysadmins, and power users, we spend most of our time in the terminal. Why should a simple search require launching a separate application, waiting for it to load, navigating through a cluttered UI, and then switching back to our work?

### The Solution

Navim brings web browsing directly into your terminal with a clean, keyboard-driven interface inspired by vim. No mouse needed. No distractions. No tracking. Just you, your query, and the information you need.

## Features

### Vim-Style Navigation
If you know vim, you already know how to use Navim. Navigate with `hjkl` or arrow keys - no mode switching required. The learning curve is zero for anyone familiar with keyboard-driven interfaces.

### In-Terminal Web Rendering
Don't just see search results - actually read the web pages. Navim converts HTML to clean, readable text rendered directly in your terminal. Articles, documentation, Stack Overflow answers - all readable without opening a browser.

### Privacy by Design
Navim uses Brave Search as its backend, which doesn't track your searches or build advertising profiles. Combined with the fact that you're not loading JavaScript, images, or third-party trackers, your searches remain truly private.

### Lightweight and Fast
Built in Rust for maximum performance. Navim launches instantly, fetches results quickly, and uses minimal system resources. No Electron, no WebKit, no bloat.

### ASCII Art Images
Navim converts images from web pages into ASCII art, so you can see visual content right in your terminal without leaving the text-based interface.

### Distraction-Free
No ads. No suggested videos. No "people also searched for" rabbit holes. Just the information you asked for, presented cleanly in your terminal.

## Installation

### Quick Install (Recommended)

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/politikl/navim/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/politikl/navim/main/install.ps1 | iex
```

After installation, add to your PATH:

**macOS / Linux** - Add this to your `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

**Windows** - Add `%USERPROFILE%\.local\bin` to your PATH via System Settings, or run in PowerShell (as Administrator):
```powershell
[Environment]::SetEnvironmentVariable('Path', $env:Path + ';' + $env:USERPROFILE + '\.local\bin', 'User')
```

### From Source

If you have Rust installed, you can build from source:

```bash
git clone https://github.com/politikl/navim.git
cd navim
cargo install --path .
```

Make sure `~/.cargo/bin` is in your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Usage

Simply run `navim` followed by your query:

```bash
navim rust programming
navim how to exit vim
navim kubernetes pod restart policy
navim best practices for API design
```

The TUI will launch showing your search results. Navigate, select, and read - all without leaving your terminal.

### Special Commands

```bash
navim about  # Show about information
navim -h     # View your browsing history
```

The `-h` flag shows all pages you've visited along with the search query that led you there and when you visited.

## Keybindings

Navim uses a simple two-view interface: Search Results and Web Page. Navigation works immediately - no mode switching required.

### Search Results View

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` / `l` / `→` | Open selected page |
| `q` / `Esc` | Quit the application |

### Web Page View

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down 1 line |
| `k` / `↑` | Scroll up 1 line |
| `Space` / `d` / `PageDown` | Scroll down 20 lines |
| `b` / `u` / `PageUp` | Scroll up 20 lines |
| `g` / `Home` | Jump to top of page |
| `G` / `End` | Jump to bottom of page |
| `q` / `Esc` / `h` / `←` | Return to search results |

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

Navim is built with a carefully selected stack of Rust crates:

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client for fetching search results and web pages |
| `scraper` | HTML parsing and CSS selector-based extraction |
| `html2text` | Converting web pages to readable terminal text |
| `ratatui` | Terminal UI framework for the interface |
| `crossterm` | Cross-platform terminal manipulation |
| `image` | Image processing for ASCII art conversion |
| `url` | URL parsing and resolution |
| `serde` | JSON serialization for history storage |
| `chrono` | DateTime handling for history timestamps |

The architecture separates concerns cleanly:
- **Search module**: Handles Brave Search queries and result parsing
- **Fetch module**: Retrieves and converts web pages to text
- **Image module**: Converts web images to ASCII art
- **UI module**: Manages the TUI state and rendering
- **Input handling**: Processes keyboard events and view navigation

## Limitations

- **No JavaScript**: Pages that require JavaScript to display content won't render properly
- **ASCII Images Only**: Images are converted to ASCII art - no full-color image rendering
- **No Forms**: You cannot submit forms or log into websites
- **No CSS Styling**: Pages are rendered as plain text without visual styling

These limitations are by design. Navim is meant for quickly finding and reading information, not for interactive web applications.

## Uninstall

**If installed via quick install:**
```bash
rm ~/.local/bin/navim
```

**If installed via cargo:**
```bash
cargo uninstall navim
# or
rm ~/.cargo/bin/navim
```

**Windows:**
```powershell
Remove-Item $env:USERPROFILE\.local\bin\navim.exe
```

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## License

MIT License - feel free to use, modify, and distribute.
