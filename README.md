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

### Home Screen
Launch Navim without any arguments to see a beautiful ASCII logo home screen with a search box. Start typing your query and press Enter to search - no command line arguments needed.

```bash
navim  # Opens the home screen with search box
```

### Full Vim-Style Navigation
If you know vim, you already know how to use Navim. Navigate web pages exactly like you would navigate a file in vim:

- **`h`/`j`/`k`/`l`** - Move cursor left/down/up/right by character
- **`w`/`b`** - Jump forward/backward by word
- **Count prefixes** - Type a number before any motion (e.g., `20j` moves down 20 lines, `5w` jumps 5 words)
- **`G`** - Jump to end of page, or `50G` to jump to line 50
- **`g`** - Jump to top of page

### Interactive Link Navigation
Browse the web like you browse code:

- **Links are highlighted** in cyan with underlines so you can see them clearly
- **`L`** - Jump to the next link on the page
- **`H`** - Jump to the previous link
- **Selected links** turn yellow so you know exactly which one you're on
- **`Enter`** - Follow the selected link to load that page

### Relative Line Numbers
Just like vim's `relativenumber` option, Navim shows:
- The **absolute line number** on the line where your cursor is
- **Relative distances** on all other lines (1, 2, 3, etc.)

This makes it easy to know exactly how far to jump - if you see `15` on a line, type `15j` to get there instantly.

### Smart Line Wrapping
Long lines that overflow the terminal width automatically wrap to new lines, each with their own line number. No horizontal scrolling needed - all content is visible and navigable.

### Visual Cursor
A **blue highlighted cursor** shows your exact position on the page. Move character by character with `h`/`l`, or jump around with word motions and line numbers.

### In-Terminal Web Rendering
Don't just see search results - actually read the web pages. Navim renders HTML into clean, readable text:

- **Headings** are formatted with visual separators (`═══`, `━━`, `──`)
- **Lists** display with bullet points and proper indentation
- **Code blocks** are wrapped in boxes for easy identification
- **Blockquotes** show with a vertical bar prefix
- **Links** are bracketed and highlighted for visibility

### ASCII Art Images
Images from web pages are converted to ASCII art and displayed inline where they appear in the document. See visual content without leaving your text-based interface.

### Privacy by Design
Navim uses Brave Search as its backend, which doesn't track your searches or build advertising profiles. Combined with the fact that you're not loading JavaScript, images, or third-party trackers, your searches remain truly private.

### Lightweight and Fast
Built in Rust for maximum performance. Navim launches instantly, fetches results quickly, and uses minimal system resources. No Electron, no WebKit, no bloat.

### Browsing History
Navim keeps a local history of pages you've visited. View it anytime with:

```bash
navim -h  # View your browsing history
```

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

### Home Screen Mode
```bash
navim  # Opens home screen with search box
```
Just start typing and press Enter to search.

### Direct Search Mode
```bash
navim rust programming
navim how to exit vim
navim kubernetes pod restart policy
navim best practices for API design
```

### Direct URL Mode
```bash
navim https://example.com  # Opens the page directly
```

### Special Commands
```bash
navim about  # Show about information
navim -h     # View your browsing history
```

## Keybindings

Navim uses a simple interface with three views: Home, Search Results, and Web Page.

### Home Screen

| Key | Action |
|-----|--------|
| Type | Enter search query |
| `Enter` | Perform search |
| `←`/`→` | Move cursor in search box |
| `Backspace` | Delete character |
| `Esc`/`q` | Quit (when search box is empty) |

### Search Results View

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` / `l` / `→` | Open selected page |
| `q` / `Esc` | Return to home screen |

### Web Page View - Cursor Movement

| Key | Action |
|-----|--------|
| `h` / `←` | Move cursor left one character |
| `l` / `→` | Move cursor right one character |
| `j` / `↓` | Move cursor down one line |
| `k` / `↑` | Move cursor up one line |
| `w` | Jump to next word |
| `b` | Jump to previous word |

### Web Page View - Link Navigation

| Key | Action |
|-----|--------|
| `L` / `Tab` | Jump to next link |
| `H` / `Shift+Tab` | Jump to previous link |
| `Enter` | Follow the selected link |

### Web Page View - Page Navigation

| Key | Action |
|-----|--------|
| `Space` / `d` / `PageDown` | Scroll down half page |
| `u` / `PageUp` | Scroll up half page |
| `g` / `Home` | Jump to top of page |
| `G` / `End` | Jump to bottom of page |
| `q` / `Esc` | Return to search results |

### Count Prefixes (Vim-Style)

Prefix any motion with a number to repeat it:

| Example | Action |
|---------|--------|
| `20j` | Move down 20 lines |
| `10k` | Move up 10 lines |
| `5w` | Jump forward 5 words |
| `3b` | Jump backward 3 words |
| `50G` | Jump to line 50 |
| `10l` | Move right 10 characters |

## How It Works

### Architecture

Navim is built with a clean separation of concerns:

1. **Home Screen**: A welcoming TUI with ASCII art logo and centered search box
2. **Search Module**: Sends queries to Brave Search and parses the HTML response to extract titles, URLs, and descriptions
3. **Page Fetcher**: Retrieves web pages and processes them through a custom HTML renderer
4. **HTML Renderer**: Walks the DOM tree in document order, converting elements to formatted text while tracking link positions
5. **Link Tracker**: Records the exact line and column positions of every link for precise cursor navigation
6. **TUI Engine**: Renders everything using ratatui with custom styling for links, cursor, and line numbers
7. **Input Handler**: Processes vim-style keybindings with support for count prefixes

### Page Rendering Pipeline

When you open a web page:

1. The HTML is fetched with a browser-like User-Agent
2. The document is parsed and the main content area is identified (article, main, etc.)
3. The custom renderer walks the DOM in order, outputting formatted text
4. Links are tracked with their exact positions (line, column start, column end)
5. Images are fetched and converted to ASCII art inline
6. The result is displayed with syntax highlighting for links and a visual cursor

### Cursor Navigation

The cursor system works like vim:

- **Desired column memory**: When moving vertically through lines of different lengths, Navim remembers where you want to be horizontally
- **Word boundaries**: `w` and `b` respect word boundaries (whitespace-delimited)
- **Link awareness**: The cursor knows when it's on a link and highlights it
- **Count multipliers**: All motions accept numeric prefixes

## Technical Details

Navim is built with a carefully selected stack of Rust crates:

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client with rustls for secure fetching |
| `scraper` | HTML parsing and DOM traversal |
| `ratatui` | Terminal UI framework for rendering |
| `crossterm` | Cross-platform terminal manipulation |
| `image` | Image processing for ASCII art conversion |
| `url` | URL parsing and resolution for relative links |
| `serde` | JSON serialization for history storage |
| `chrono` | DateTime handling for history timestamps |
| `dirs` | Cross-platform config directory detection |
| `colored` | Terminal color support |

## Use Cases

### For Developers
- Quickly look up documentation without leaving your editor
- Search Stack Overflow while debugging in the terminal
- Check API references without context switching
- Read technical articles with proper code block formatting

### For System Administrators
- Look up command syntax and options
- Search for troubleshooting guides while SSHed into a server
- Find configuration examples without a GUI
- Works over SSH - no X forwarding needed

### For Privacy-Conscious Users
- Search without being tracked or profiled
- No cookies, no JavaScript, no tracking pixels
- Local history only - nothing sent to third parties
- No personalized results based on past behavior

### For Productivity Enthusiasts
- Stay in your terminal flow state
- Vim keybindings mean muscle memory transfers
- No visual distractions or clickbait
- Count prefixes make navigation lightning fast

## Limitations

- **No JavaScript**: Pages that require JavaScript to display content won't render properly
- **ASCII Images Only**: Images are converted to ASCII art - no full-color image rendering
- **No Forms**: You cannot submit forms or log into websites
- **No CSS Styling**: Pages are rendered as structured text without visual styling
- **Read-Only**: This is for consuming content, not interacting with web apps

These limitations are by design. Navim is meant for quickly finding and reading information, not for interactive web applications.

## Configuration

Navim stores its configuration and history in:
- **macOS/Linux**: `~/.config/navim/`
- **Windows**: `%APPDATA%\navim\`

History is stored in `history.json` and keeps the last 100 visited pages.

## Uninstall

**If installed via quick install:**
```bash
rm ~/.local/bin/navim
rm -rf ~/.config/navim  # Remove history
```

**If installed via cargo:**
```bash
cargo uninstall navim
rm -rf ~/.config/navim  # Remove history
```

**Windows:**
```powershell
Remove-Item $env:USERPROFILE\.local\bin\navim.exe
Remove-Item -Recurse $env:APPDATA\navim  # Remove history
```

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## License

MIT License - feel free to use, modify, and distribute.

## Version History

### 2.0.0
- Added home screen with ASCII logo and search box
- Full vim-style cursor navigation (h/j/k/l)
- Word movement with w/b
- Interactive link navigation with L/H
- Visual cursor highlighting
- Relative line numbers
- Count prefix support (20j, 5w, 50G, etc.)
- Smart line wrapping
- Improved HTML rendering with proper document structure
- Links tracked and highlighted inline

### 1.x
- Initial release with search and page viewing
- Basic vim-style scrolling
- ASCII art image support
- Browsing history
