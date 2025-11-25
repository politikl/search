# Search - Terminal Web Browser

**A full terminal-based web browser with vim-style navigation.** Search the web and view pages without leaving your terminal.

## Features

- **Vim-style Navigation**: Use `i` for insert mode, `hjkl` to navigate, `Esc` for normal mode
- **In-Terminal Web Viewing**: Render web pages as text directly in the terminal
- **TUI Interface**: Beautiful terminal UI powered by ratatui
- **Fast**: Built in Rust for speed
- **Privacy-Focused**: Uses Brave Search, no tracking

## Installation

```bash
git clone https://github.com/politikl/search.git
cd search
cargo install --path .
```

Make sure `~/.cargo/bin` is in your PATH.

## Usage

```bash
search rust programming
search how to cook pasta
search best laptop 2024
```

## Keybindings

### Search Results View

| Mode | Key | Action |
|------|-----|--------|
| NORMAL | `i` | Enter INSERT mode |
| NORMAL | `q` | Quit |
| INSERT | `j` / `k` | Navigate down/up |
| INSERT | `h` / `l` | Navigate up/down |
| INSERT | `Enter` | Open page in terminal |
| INSERT | `Esc` | Return to NORMAL mode |

### Web Page View

| Mode | Key | Action |
|------|-----|--------|
| NORMAL | `i` | Enter BROWSE mode |
| NORMAL | `q` / `Esc` | Back to search results |
| BROWSE | `j` / `k` | Scroll down/up 1 line |
| BROWSE | `J` / `K` | Scroll down/up 10 lines |
| BROWSE | `d` / `u` | Scroll down/up 10 lines |
| BROWSE | `g` | Go to top |
| BROWSE | `G` | Go to bottom |
| BROWSE | `q` | Back to search results |

## How It Works

1. Search queries are sent to Brave Search
2. Results are displayed in a TUI list
3. Select a result and press Enter to view the page
4. Web pages are converted to plain text using html2text
5. Navigate the page with vim-style keybindings
6. Press `q` to return to search results

## Technical Details

- Built with Rust
- `reqwest` - HTTP requests
- `scraper` - HTML parsing for search results
- `html2text` - Convert web pages to terminal text
- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation

## Uninstall

```bash
cargo uninstall search
```
