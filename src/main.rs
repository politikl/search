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
    Home,
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
    // Home screen
    search_input: String,
    cursor_position: usize,
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
            search_input: String::new(),
            cursor_position: 0,
        }
    }

    fn new_home() -> Self {
        App {
            results: Vec::new(),
            list_state: ListState::default(),
            view: View::Home,
            query: String::new(),
            should_quit: false,
            page_content: Vec::new(),
            page_scroll: 0,
            page_title: String::new(),
            page_url: String::new(),
            search_input: String::new(),
            cursor_position: 0,
        }
    }

    fn insert_char(&mut self, c: char) {
        self.search_input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.search_input.remove(self.cursor_position);
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.search_input.len() {
            self.cursor_position += 1;
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

// Check if an image URL should be rendered
fn should_render_image(src: &str) -> bool {
    let src_lower = src.to_lowercase();

    !(src_lower.starts_with("data:")
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
        || src_lower.contains("widget")
        || src_lower.contains("logo")
        || src_lower.contains("spinner")
        || src_lower.contains("loading"))
}

// Resolve a potentially relative URL to absolute
fn resolve_url(src: &str, base: &Option<Url>) -> Option<String> {
    if src.starts_with("http") {
        Some(src.to_string())
    } else if src.starts_with("//") {
        Some(format!("https:{}", src))
    } else if let Some(ref base) = base {
        base.join(src).map(|u| u.to_string()).ok()
    } else {
        None
    }
}

// Custom HTML renderer that preserves document structure
struct HtmlRenderer {
    output: String,
    base_url: Option<Url>,
    image_count: usize,
    max_images: usize,
    list_depth: usize,
    in_pre: bool,
    last_was_block: bool,
}

impl HtmlRenderer {
    fn new(base_url: &str) -> Self {
        HtmlRenderer {
            output: String::new(),
            base_url: Url::parse(base_url).ok(),
            image_count: 0,
            max_images: 3,
            list_depth: 0,
            in_pre: false,
            last_was_block: true,
        }
    }

    fn render_element(&mut self, element: scraper::ElementRef) {
        let tag = element.value().name();

        // Skip unwanted elements
        if matches!(tag, "script" | "style" | "nav" | "header" | "footer" | "aside" | "noscript" | "iframe" | "form") {
            return;
        }

        // Skip elements with hidden classes
        if let Some(class) = element.value().attr("class") {
            let class_lower = class.to_lowercase();
            if class_lower.contains("hidden") || class_lower.contains("sidebar")
                || class_lower.contains("nav") || class_lower.contains("menu")
                || class_lower.contains("footer") || class_lower.contains("header")
                || class_lower.contains("advertisement") || class_lower.contains("ad-") {
                return;
            }
        }

        match tag {
            // Block elements that need newlines
            "p" => {
                self.ensure_blank_line();
                self.render_children(element);
                self.ensure_newline();
                self.last_was_block = true;
            }
            "div" | "section" | "article" => {
                self.ensure_newline();
                self.render_children(element);
                self.ensure_newline();
                self.last_was_block = true;
            }
            "br" => {
                self.output.push('\n');
                self.last_was_block = true;
            }
            "hr" => {
                self.ensure_blank_line();
                self.output.push_str("────────────────────────────────────────");
                self.ensure_blank_line();
                self.last_was_block = true;
            }

            // Headings
            "h1" => {
                self.ensure_blank_line();
                self.output.push_str("═══ ");
                self.render_children(element);
                self.output.push_str(" ═══");
                self.ensure_blank_line();
                self.last_was_block = true;
            }
            "h2" => {
                self.ensure_blank_line();
                self.output.push_str("━━ ");
                self.render_children(element);
                self.output.push_str(" ━━");
                self.ensure_blank_line();
                self.last_was_block = true;
            }
            "h3" => {
                self.ensure_blank_line();
                self.output.push_str("── ");
                self.render_children(element);
                self.output.push_str(" ──");
                self.ensure_blank_line();
                self.last_was_block = true;
            }
            "h4" | "h5" | "h6" => {
                self.ensure_blank_line();
                self.output.push_str("▸ ");
                self.render_children(element);
                self.ensure_newline();
                self.last_was_block = true;
            }

            // Lists
            "ul" | "ol" => {
                self.ensure_newline();
                self.list_depth += 1;
                self.render_children(element);
                self.list_depth -= 1;
                self.ensure_newline();
                self.last_was_block = true;
            }
            "li" => {
                self.ensure_newline();
                let indent = "  ".repeat(self.list_depth.saturating_sub(1));
                self.output.push_str(&indent);
                self.output.push_str("• ");
                self.render_children(element);
                self.last_was_block = true;
            }

            // Code and preformatted
            "pre" => {
                self.ensure_blank_line();
                self.output.push_str("┌─────────────────────────────────────────┐\n");
                self.in_pre = true;
                self.render_children(element);
                self.in_pre = false;
                self.ensure_newline();
                self.output.push_str("└─────────────────────────────────────────┘");
                self.ensure_blank_line();
                self.last_was_block = true;
            }
            "code" => {
                if !self.in_pre {
                    self.output.push('`');
                    self.render_children(element);
                    self.output.push('`');
                } else {
                    self.render_children(element);
                }
            }

            // Inline formatting
            "strong" | "b" => {
                self.output.push_str("**");
                self.render_children(element);
                self.output.push_str("**");
            }
            "em" | "i" => {
                self.output.push('_');
                self.render_children(element);
                self.output.push('_');
            }

            // Links
            "a" => {
                self.render_children(element);
                if let Some(href) = element.value().attr("href") {
                    if href.starts_with("http") && !href.contains("javascript:") {
                        self.output.push_str(" [→ ");
                        self.output.push_str(&truncate_string(href, 40));
                        self.output.push(']');
                    }
                }
            }

            // Images - render inline where they appear
            "img" => {
                if self.image_count < self.max_images {
                    let src = element.value().attr("src")
                        .or_else(|| element.value().attr("data-src"))
                        .or_else(|| element.value().attr("data-lazy-src"));

                    if let Some(src) = src {
                        if should_render_image(src) {
                            if let Some(full_url) = resolve_url(src, &self.base_url) {
                                if let Some(ascii_art) = fetch_image_as_ascii(&full_url, 60) {
                                    self.ensure_blank_line();
                                    // Add image caption if available
                                    if let Some(alt) = element.value().attr("alt") {
                                        if !alt.is_empty() && alt.len() < 100 {
                                            self.output.push_str(&format!("[Image: {}]\n", alt));
                                        }
                                    }
                                    self.output.push_str(&ascii_art);
                                    self.ensure_blank_line();
                                    self.image_count += 1;
                                    self.last_was_block = true;
                                }
                            }
                        }
                    }
                }
            }

            // Figure (often wraps images with captions)
            "figure" => {
                self.ensure_newline();
                self.render_children(element);
                self.ensure_newline();
                self.last_was_block = true;
            }
            "figcaption" => {
                self.output.push_str("  ↳ ");
                self.render_children(element);
                self.ensure_newline();
            }

            // Blockquote
            "blockquote" => {
                self.ensure_blank_line();
                let start_len = self.output.len();
                self.render_children(element);
                // Add quote markers to each line
                let content = self.output[start_len..].to_string();
                self.output.truncate(start_len);
                for line in content.lines() {
                    self.output.push_str("│ ");
                    self.output.push_str(line);
                    self.output.push('\n');
                }
                self.last_was_block = true;
            }

            // Tables - simplified rendering
            "table" => {
                self.ensure_blank_line();
                self.render_children(element);
                self.ensure_blank_line();
                self.last_was_block = true;
            }
            "tr" => {
                self.ensure_newline();
                self.output.push_str("│ ");
                self.render_children(element);
                self.last_was_block = true;
            }
            "th" | "td" => {
                self.render_children(element);
                self.output.push_str(" │ ");
            }

            // Span and other inline elements
            "span" | "label" | "time" | "small" | "sup" | "sub" => {
                self.render_children(element);
            }

            // Default: just render children
            _ => {
                self.render_children(element);
            }
        }
    }

    fn render_children(&mut self, element: scraper::ElementRef) {
        for child in element.children() {
            match child.value() {
                scraper::Node::Text(text) => {
                    let content = text.text.to_string();
                    if self.in_pre {
                        self.output.push_str(&content);
                    } else {
                        // Normalize whitespace
                        let normalized: String = content
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ");
                        if !normalized.is_empty() {
                            if !self.last_was_block && !self.output.is_empty()
                                && !self.output.ends_with(' ') && !self.output.ends_with('\n') {
                                self.output.push(' ');
                            }
                            self.output.push_str(&normalized);
                            self.last_was_block = false;
                        }
                    }
                }
                scraper::Node::Element(_) => {
                    if let Some(child_elem) = scraper::ElementRef::wrap(child) {
                        self.render_element(child_elem);
                    }
                }
                _ => {}
            }
        }
    }

    fn ensure_newline(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn ensure_blank_line(&mut self) {
        self.ensure_newline();
        if !self.output.ends_with("\n\n") {
            self.output.push('\n');
        }
    }

    fn finish(self) -> String {
        self.output
    }
}

// Extract and render content with proper structure
fn extract_content_with_images(html: &str, base_url: &str) -> String {
    let document = Html::parse_document(html);

    // Find the main content element
    let content_selectors = vec![
        "#mw-content-text .mw-parser-output",
        "#mw-content-text",
        "#bodyContent",
        ".question .s-prose",
        ".answercell .s-prose",
        "#mainbar",
        "article .post-content",
        "article .entry-content",
        "article .content",
        "article",
        "main .content",
        "main article",
        "#main-content",
        ".main-content",
        "[role='main']",
        "main",
        ".post-body",
        ".article-body",
        ".story-body",
        ".markdown-body",
        ".documentation",
        ".doc-content",
        "#readme",
        "#content",
        "body",
    ];

    // Find the best content element
    let mut content_element = None;
    for sel_str in &content_selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            if let Some(element) = document.select(&selector).next() {
                let inner = element.html();
                if inner.len() > 500 {
                    content_element = Some(element);
                    break;
                }
            }
        }
    }

    // Render the content
    let mut renderer = HtmlRenderer::new(base_url);

    if let Some(element) = content_element {
        renderer.render_element(element);
    } else {
        // Fallback to body
        if let Ok(selector) = Selector::parse("body") {
            if let Some(body) = document.select(&selector).next() {
                renderer.render_element(body);
            }
        }
    }

    renderer.finish()
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

    // Extract content with images placed inline
    let text = extract_content_with_images(&html, url);

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

fn draw_home(f: &mut ratatui::Frame, app: &mut App) {
    use ratatui::layout::Alignment;

    let area = f.area();

    // Calculate vertical centering
    let logo_height = 8;
    let search_box_height = 3;
    let tips_height = 5;
    let total_content_height = logo_height + search_box_height + tips_height + 4; // +4 for spacing
    let vertical_padding = area.height.saturating_sub(total_content_height) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_padding),
            Constraint::Length(logo_height),
            Constraint::Length(2),
            Constraint::Length(search_box_height),
            Constraint::Length(2),
            Constraint::Length(tips_height),
            Constraint::Min(0),
        ])
        .split(area);

    // ASCII Art Logo - centered
    let ascii_logo = vec![
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
        Line::from(""),
        Line::from(Span::styled(
            "Terminal Web Browser",
            Style::default().fg(Color::Gray),
        )),
    ];

    let logo = Paragraph::new(ascii_logo).alignment(Alignment::Center);
    f.render_widget(logo, chunks[1]);

    // Search box - centered with fixed width
    let search_width = 60.min(area.width.saturating_sub(4));
    let search_padding = (area.width.saturating_sub(search_width)) / 2;

    let search_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(search_padding),
            Constraint::Length(search_width),
            Constraint::Min(0),
        ])
        .split(chunks[3]);

    // Build search input with cursor
    let input_text = if app.search_input.is_empty() {
        Span::styled("Search the web...", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&app.search_input, Style::default().fg(Color::White))
    };

    let search_box = Paragraph::new(Line::from(input_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Span::styled(
                    " Search ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(search_box, search_area[1]);

    // Set cursor position
    let cursor_x = search_area[1].x + 1 + app.cursor_position as u16;
    let cursor_y = search_area[1].y + 1;
    f.set_cursor_position((cursor_x, cursor_y));

    // Tips/help text - centered
    let tips = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" to search  ", Style::default().fg(Color::Gray)),
            Span::styled("Esc/q", Style::default().fg(Color::Yellow)),
            Span::styled(" to quit", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Privacy-focused browsing powered by Brave Search",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let tips_widget = Paragraph::new(tips).alignment(Alignment::Center);
    f.render_widget(tips_widget, chunks[5]);
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<Result<(), Box<dyn Error>>> {
    loop {
        terminal.draw(|f| {
            match app.view {
                View::Home => draw_home(f, &mut app),
                View::SearchResults => draw_search_results(f, &mut app),
                View::WebPage => draw_web_page(f, &mut app),
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match &app.view {
                    // Home screen - text input mode
                    View::Home => match code {
                        KeyCode::Esc => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('q') if app.search_input.is_empty() => {
                            app.should_quit = true;
                        }
                        KeyCode::Enter => {
                            if !app.search_input.is_empty() {
                                // Perform search
                                let query = app.search_input.clone();
                                match search(&query) {
                                    Ok(results) => {
                                        if results.is_empty() {
                                            // Stay on home, could show "no results" message
                                        } else {
                                            app.results = results;
                                            app.query = query;
                                            app.list_state = ListState::default();
                                            app.list_state.select(Some(0));
                                            app.view = View::SearchResults;
                                        }
                                    }
                                    Err(e) => {
                                        return Ok(Err(e));
                                    }
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            app.insert_char(c);
                        }
                        KeyCode::Backspace => {
                            app.delete_char();
                        }
                        KeyCode::Left => {
                            app.move_cursor_left();
                        }
                        KeyCode::Right => {
                            app.move_cursor_right();
                        }
                        _ => {}
                    },
                    // Search Results - navigation works immediately
                    View::SearchResults => match code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            // Go back to home instead of quitting
                            app.view = View::Home;
                            app.results.clear();
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
            return Ok(Ok(()));
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
                    "  Version 1.3.1",
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

    // No arguments - show home screen
    if args.len() < 2 {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let app = App::new_home();
        let res = run_app(&mut terminal, app);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        match res {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(Box::new(e)),
        }
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

    match res {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(Box::new(e)),
    }
}
