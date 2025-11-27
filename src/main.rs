use chrono::{DateTime, Local};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use image::GenericImageView;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

fn truncate_string(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

fn sanitize_display(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == ' ' || *c == '\n')
        .collect::<String>()
        .trim()
        .to_string()
}

// ASCII art characters from darkest to lightest
const ASCII_CHARS: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

// Convert image bytes to ASCII art
fn image_to_ascii(image_bytes: &[u8], max_width: u32) -> Option<String> {
    let img = image::load_from_memory(image_bytes).ok()?;

    let (width, height) = img.dimensions();

    // Calculate new dimensions while maintaining aspect ratio
    // Terminal characters are roughly 2x taller than wide, so we adjust
    let aspect_ratio = height as f32 / width as f32;
    let new_width = max_width.min(width);
    let new_height = ((new_width as f32 * aspect_ratio) / 2.0) as u32;

    // Skip very small or very large images
    if new_width < 10 || new_height < 5 || new_height > 50 {
        return None;
    }

    let resized = img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);
    let gray = resized.to_luma8();

    let mut ascii_art = String::new();
    ascii_art.push_str("\n┌");
    for _ in 0..new_width {
        ascii_art.push('─');
    }
    ascii_art.push_str("┐\n");

    for y in 0..new_height {
        ascii_art.push('│');
        for x in 0..new_width {
            let pixel = gray.get_pixel(x, y);
            let brightness = pixel[0] as usize;
            let char_index = (brightness * (ASCII_CHARS.len() - 1)) / 255;
            ascii_art.push(ASCII_CHARS[char_index]);
        }
        ascii_art.push_str("│\n");
    }

    ascii_art.push_str("└");
    for _ in 0..new_width {
        ascii_art.push('─');
    }
    ascii_art.push_str("┘\n");

    Some(ascii_art)
}

// Fetch an image and convert to ASCII
fn fetch_image_as_ascii(image_url: &str, max_width: u32) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let response = client.get(image_url).send().ok()?;

    // Check content type to ensure it's an image
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.starts_with("image/") {
        return None;
    }

    // Skip SVG and GIF (animated)
    if content_type.contains("svg") || content_type.contains("gif") {
        return None;
    }

    let bytes = response.bytes().ok()?;

    // Skip very small images (likely icons/tracking pixels)
    if bytes.len() < 1000 {
        return None;
    }

    image_to_ascii(&bytes, max_width)
}

// Extract image URLs from HTML (focused on main content area)
fn extract_image_urls(html: &str, base_url: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let base = Url::parse(base_url).ok();

    let mut urls = Vec::new();

    // Try to find images within main content areas first
    let content_selectors = [
        "article img",
        "main img",
        "#content img",
        "#mw-content-text img",  // Wikipedia
        ".post-content img",
        ".entry-content img",
        ".article-body img",
        "img",  // Fallback to all images
    ];

    let mut found_images = Vec::new();
    for sel_str in &content_selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            for img in document.select(&selector).take(15) {
                found_images.push(img);
            }
            if found_images.len() >= 5 {
                break;  // Found enough images in content area
            }
        }
    }

    // Process found images
    for img in found_images.iter().take(20) {
        // Try src first, then data-src (lazy loading)
        let src = img.value().attr("src")
            .or_else(|| img.value().attr("data-src"))
            .or_else(|| img.value().attr("data-lazy-src"));

        if let Some(src) = src {
            let src_lower = src.to_lowercase();

            // Skip data URLs and common non-content images
            if src_lower.starts_with("data:")
                || src_lower.contains("icon")
                || src_lower.contains("avatar")
                || src_lower.contains("sprite")
                || src_lower.contains("tracking")
                || src_lower.contains("pixel")
                || src_lower.contains("1x1")
                || src_lower.contains("badge")
                || src_lower.contains("button")
                || src_lower.contains("arrow")
                || src_lower.contains("spacer")
                || src_lower.ends_with(".svg")
                || src_lower.ends_with(".gif")
                || src_lower.contains("/static/")
                || src_lower.contains("widget") {
                continue;
            }

            // Resolve relative URLs
            let full_url = if src.starts_with("http") {
                src.to_string()
            } else if src.starts_with("//") {
                format!("https:{}", src)
            } else if let Some(ref base) = base {
                base.join(src).map(|u| u.to_string()).unwrap_or_default()
            } else {
                continue;
            };

            if !full_url.is_empty() && !urls.contains(&full_url) {
                urls.push(full_url);
            }
        }
    }

    urls
}

// History functionality
#[derive(Serialize, Deserialize, Clone)]
struct HistoryEntry {
    query: String,
    title: String,
    url: String,
    timestamp: DateTime<Local>,
}

fn get_history_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("navim");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("history.json")
}

fn load_history() -> Vec<HistoryEntry> {
    let path = get_history_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_history(history: &[HistoryEntry]) {
    let path = get_history_path();
    if let Ok(json) = serde_json::to_string_pretty(history) {
        fs::write(&path, json).ok();
    }
}

fn add_to_history(query: &str, title: &str, url: &str) {
    let mut history = load_history();
    history.insert(
        0,
        HistoryEntry {
            query: query.to_string(),
            title: title.to_string(),
            url: url.to_string(),
            timestamp: Local::now(),
        },
    );
    // Keep only the last 100 entries
    history.truncate(100);
    save_history(&history);
}

#[derive(Clone)]
struct SearchResult {
    title: String,
    url: String,
    display_url: String,
    description: String,
}

#[derive(PartialEq, Clone)]
enum View {
    SearchResults,
    WebPage,
}

struct App {
    results: Vec<SearchResult>,
    list_state: ListState,
    view: View,
    query: String,
    should_quit: bool,
    // Web page viewing
    page_content: Vec<String>,
    page_scroll: usize,
    page_title: String,
    page_url: String,
}

impl App {
    fn new(results: Vec<SearchResult>, query: String) -> Self {
        let mut list_state = ListState::default();
        if !results.is_empty() {
            list_state.select(Some(0));
        }
        App {
            results,
            list_state,
            view: View::SearchResults,
            query,
            should_quit: false,
            page_content: Vec::new(),
            page_scroll: 0,
            page_title: String::new(),
            page_url: String::new(),
        }
    }

    fn next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.results.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.results.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn scroll_down(&mut self, amount: usize) {
        if self.page_scroll + amount < self.page_content.len().saturating_sub(10) {
            self.page_scroll += amount;
        } else {
            self.page_scroll = self.page_content.len().saturating_sub(10);
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        self.page_scroll = self.page_scroll.saturating_sub(amount);
    }

    fn open_selected(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if let Some(result) = self.results.get(i) {
                if !result.url.is_empty() {
                    self.page_title = result.title.clone();
                    self.page_url = result.url.clone();
                    self.page_scroll = 0;

                    // Save to history
                    add_to_history(&self.query, &result.title, &result.url);

                    // Fetch and render the page
                    match fetch_page(&result.url) {
                        Ok(content) => {
                            self.page_content = content.lines().map(|s| s.to_string()).collect();
                            self.view = View::WebPage;
                        }
                        Err(_) => {
                            self.page_content = vec!["Failed to load page.".to_string()];
                            self.view = View::WebPage;
                        }
                    }
                }
            }
        }
    }

    fn back_to_results(&mut self) {
        self.view = View::SearchResults;
        self.page_content.clear();
        self.page_scroll = 0;
    }
}

fn extract_main_content(html_text: &str) -> String {
    let document = Html::parse_document(html_text);

    // Site-specific selectors in priority order - targets main content, skips nav/sidebars
    let selectors = vec![
        // Wikipedia - the actual article content
        "#mw-content-text .mw-parser-output",
        "#mw-content-text",
        "#bodyContent",

        // StackOverflow
        ".question .s-prose",
        ".answercell .s-prose",
        "#mainbar",

        // Generic article selectors
        "article .post-content",
        "article .entry-content",
        "article .content",
        "article",

        // Main content areas
        "main .content",
        "main article",
        "#main-content",
        ".main-content",
        "[role='main']",
        "main",

        // Blog/news sites
        ".post-body",
        ".article-body",
        ".story-body",

        // Documentation sites
        ".markdown-body",
        ".documentation",
        ".doc-content",
        "#readme",
        "#content",
    ];

    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            if let Some(element) = document.select(&selector).next() {
                let inner_html = element.html();
                // Skip if content is too short (probably wrong element)
                if inner_html.len() > 500 {
                    return html2text::from_read(inner_html.as_bytes(), 100);
                }
            }
        }
    }

    // Fallback: use full page
    html2text::from_read(html_text.as_bytes(), 100)
}

fn fetch_page(url: &str) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(15))
        .build()?;

    let response = client
        .get(url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .send()?;

    let html = response.text()?;

    // Extract image URLs before converting to text
    let image_urls = extract_image_urls(&html, url);

    // Extract main content and convert to plain text
    let mut text = extract_main_content(&html);

    // Fetch and render images as ASCII art (limit to first 3 to keep it reasonable)
    let mut ascii_images = String::new();
    for img_url in image_urls.iter().take(3) {
        if let Some(ascii_art) = fetch_image_as_ascii(img_url, 60) {
            ascii_images.push_str(&ascii_art);
            ascii_images.push('\n');
        }
    }

    // Prepend images to content if any were found
    if !ascii_images.is_empty() {
        text = format!("{}\n{}", ascii_images, text);
    }

    Ok(sanitize_display(&text))
}

fn search(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let encoded_query = query.replace(" ", "+");
    let url = format!("https://search.brave.com/search?q={}", encoded_query);

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(15))
        .build()?;

    let response = client
        .get(&url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .send()?;
    let body = response.text()?;
    let document = Html::parse_document(&body);

    let mut results = Vec::new();

    let snippet_selector = Selector::parse("div.snippet").unwrap();
    let title_link_selector = Selector::parse("a.heading-serpresult, a[href]").unwrap();
    let title_selector = Selector::parse(".title").unwrap();
    let url_selector = Selector::parse(".snippet-url").unwrap();
    let desc_selector = Selector::parse(".snippet-description, .generic-snippet").unwrap();

    for snippet in document.select(&snippet_selector).take(10) {
        let title = snippet
            .select(&title_selector)
            .next()
            .map(|e| sanitize_display(&e.text().collect::<String>()))
            .unwrap_or_default();

        let actual_url = snippet
            .select(&title_link_selector)
            .find(|e| {
                e.value()
                    .attr("href")
                    .map(|h| h.starts_with("http"))
                    .unwrap_or(false)
            })
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default()
            .to_string();

        let display_url = snippet
            .select(&url_selector)
            .next()
            .map(|e| {
                e.text()
                    .collect::<String>()
                    .replace("›", "/")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .unwrap_or_default();

        let description = snippet
            .select(&desc_selector)
            .next()
            .map(|e| sanitize_display(&e.text().collect::<String>()))
            .unwrap_or_default();

        if !title.is_empty() && !actual_url.is_empty() {
            results.push(SearchResult {
                title: title.trim().to_string(),
                url: actual_url,
                display_url: display_url.trim().to_string(),
                description: description.trim().to_string(),
            });
        }
    }

    Ok(results)
}

fn draw_search_results(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " NAVIM ",
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  Search: "),
        Span::styled(&app.query, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Search"));
    f.render_widget(header, chunks[0]);

    // Results list
    let items: Vec<ListItem> = app
        .results
        .iter()
        .map(|r| {
            let lines = vec![
                Line::from(Span::styled(
                    truncate_string(&r.title, 70),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    truncate_string(&r.display_url, 60),
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(Span::styled(
                    truncate_string(&r.description, 80),
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
            ];
            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Results"))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // Footer with intuitive keys
    let footer = Paragraph::new(" ↑/↓ or j/k: Navigate  Enter: Open  q: Quit ")
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Keys"));
    f.render_widget(footer, chunks[2]);
}

fn draw_web_page(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header with page info
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " READING ",
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            truncate_string(&app.page_title, 50),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL).title(truncate_string(&app.page_url, 60)));
    f.render_widget(header, chunks[0]);

    // Page content
    let visible_height = chunks[1].height.saturating_sub(2) as usize;
    let content_lines: Vec<Line> = app
        .page_content
        .iter()
        .skip(app.page_scroll)
        .take(visible_height)
        .map(|line| Line::from(Span::raw(line.as_str())))
        .collect();

    let scroll_info = format!(
        " Line {}/{} ",
        app.page_scroll + 1,
        app.page_content.len().max(1)
    );

    let page = Paragraph::new(content_lines)
        .block(Block::default().borders(Borders::ALL).title(scroll_info))
        .wrap(Wrap { trim: false });
    f.render_widget(page, chunks[1]);

    // Footer with intuitive keys
    let footer = Paragraph::new(" ↑/↓ or j/k: Scroll  Space/b: Page Down/Up  g/G: Top/Bottom  Esc/q: Back ")
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Keys"));
    f.render_widget(footer, chunks[2]);
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            match app.view {
                View::SearchResults => draw_search_results(f, &mut app),
                View::WebPage => draw_web_page(f, &mut app),
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match &app.view {
                    // Search Results - navigation works immediately
                    View::SearchResults => match code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.previous();
                        }
                        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                            app.open_selected();
                        }
                        _ => {}
                    },
                    // Web Page - scrolling works immediately
                    View::WebPage => match code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                            app.back_to_results();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.scroll_down(1);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.scroll_up(1);
                        }
                        KeyCode::Char(' ') | KeyCode::Char('d') | KeyCode::PageDown => {
                            app.scroll_down(20);
                        }
                        KeyCode::Char('b') | KeyCode::Char('u') | KeyCode::PageUp => {
                            app.scroll_up(20);
                        }
                        KeyCode::Char('g') | KeyCode::Home => {
                            app.page_scroll = 0;
                        }
                        KeyCode::Char('G') | KeyCode::End => {
                            app.page_scroll = app.page_content.len().saturating_sub(10);
                        }
                        _ => {}
                    },
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn show_about() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // ASCII Art Header
            let ascii_art = vec![
                Line::from(Span::styled("", Style::default())),
                Line::from(Span::styled(
                    "                    _            ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "  _ __   __ ___   _(_)_ __ ___   ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    " | '_ \\ / _` \\ \\ / / | '_ ` _ \\  ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    " | | | | (_| |\\ V /| | | | | | | ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    " |_| |_|\\__,_| \\_/ |_|_| |_| |_| ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled("", Style::default())),
            ];
            let header = Paragraph::new(ascii_art)
                .block(Block::default().borders(Borders::ALL).title("Navim"));
            f.render_widget(header, chunks[0]);

            // About content
            let about_content = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Terminal Web Browser",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "  Version 1.3.0",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  A vim-style terminal browser for searching and reading the web.",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled("  FEATURES", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  - Intuitive keyboard navigation (arrows or hjkl)", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - In-terminal web page rendering", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - ASCII art image rendering", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - Privacy-focused with Brave Search backend", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - No tracking, no cookies, no JavaScript", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - Lightweight and fast (built in Rust)", Style::default().fg(Color::White))),
                Line::from(""),
                Line::from(Span::styled("  WHY NAVIM?", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  Modern browsers are bloated, track everything you do, and pull", Style::default().fg(Color::White))),
                Line::from(Span::styled("  you out of your terminal workflow. Navim lets you find and read", Style::default().fg(Color::White))),
                Line::from(Span::styled("  information without leaving the command line.", Style::default().fg(Color::White))),
                Line::from(""),
                Line::from(Span::styled("  KEYBINDINGS", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  ↑/↓ or j/k   Navigate / Scroll", Style::default().fg(Color::White))),
                Line::from(Span::styled("  Enter or →   Open selected result", Style::default().fg(Color::White))),
                Line::from(Span::styled("  ← or Esc     Go back", Style::default().fg(Color::White))),
                Line::from(Span::styled("  Space / b    Page down / up", Style::default().fg(Color::White))),
                Line::from(Span::styled("  g / G        Jump to top / bottom", Style::default().fg(Color::White))),
                Line::from(Span::styled("  q            Quit", Style::default().fg(Color::White))),
                Line::from(""),
                Line::from(Span::styled("  TECHNICAL", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  Built with: Rust, ratatui, reqwest, scraper, html2text", Style::default().fg(Color::White))),
                Line::from(Span::styled("  Source: github.com/politikl/navim", Style::default().fg(Color::Cyan))),
                Line::from(""),
                Line::from(Span::styled("  LICENSE", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  MIT License - Free and open source", Style::default().fg(Color::White))),
                Line::from(""),
            ];
            let about = Paragraph::new(about_content)
                .block(Block::default().borders(Borders::ALL).title("About"))
                .wrap(Wrap { trim: false });
            f.render_widget(about, chunks[1]);

            // Footer
            let footer = Paragraph::new(" Press [q] or [Esc] to exit ")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn show_history() -> Result<(), Box<dyn Error>> {
    let history = load_history();

    if history.is_empty() {
        println!("No history yet. Browse some pages to build your history.");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut list_state = ListState::default();
    list_state.select(Some(0));
    let mut scroll_offset = 0usize;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Header
            let header = Paragraph::new(Line::from(vec![
                Span::styled(
                    " HISTORY ",
                    Style::default()
                        .bg(Color::Magenta)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  {} entries", history.len())),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Search History"));
            f.render_widget(header, chunks[0]);

            // History list
            let visible_height = chunks[1].height.saturating_sub(2) as usize;
            let items: Vec<ListItem> = history
                .iter()
                .skip(scroll_offset)
                .take(visible_height / 4 + 1)
                .map(|entry| {
                    let lines = vec![
                        Line::from(vec![
                            Span::styled(
                                truncate_string(&entry.title, 60),
                                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("  Query: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                truncate_string(&entry.query, 50),
                                Style::default().fg(Color::Yellow),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("  URL: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                truncate_string(&entry.url, 55),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("  ", Style::default()),
                            Span::styled(
                                entry.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]),
                    ];
                    ListItem::new(lines)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    " Showing {}-{} of {} ",
                    scroll_offset + 1,
                    (scroll_offset + visible_height / 4 + 1).min(history.len()),
                    history.len()
                )))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list, chunks[1], &mut list_state);

            // Footer
            let footer = Paragraph::new(" [j/k] Navigate  [q/Esc] Exit ")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL).title("Keys"));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('j') | KeyCode::Down => {
                        if scroll_offset < history.len().saturating_sub(1) {
                            scroll_offset += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        scroll_offset = scroll_offset.saturating_sub(1);
                    }
                    KeyCode::Char('J') => {
                        scroll_offset = (scroll_offset + 5).min(history.len().saturating_sub(1));
                    }
                    KeyCode::Char('K') => {
                        scroll_offset = scroll_offset.saturating_sub(5);
                    }
                    KeyCode::Char('g') => {
                        scroll_offset = 0;
                    }
                    KeyCode::Char('G') => {
                        scroll_offset = history.len().saturating_sub(1);
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: navim <query>");
        eprintln!("       navim about  - Show about information");
        eprintln!("       navim -h     - Show browsing history");
        eprintln!("Example: navim rust programming");
        std::process::exit(1);
    }

    let query = args[1..].join(" ");

    // Check for about command
    if query.to_lowercase() == "about" {
        return show_about();
    }

    // Check for history command
    if query == "-h" {
        return show_history();
    }

    println!("Searching for: {}...", query);

    let results = search(&query)?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let app = App::new(results, query);
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {}", err);
    }

    Ok(())
}
