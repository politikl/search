use chrono::{DateTime, Local};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
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
        .join("search");
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
enum Mode {
    Normal,
    Insert,
}

#[derive(PartialEq, Clone)]
enum View {
    SearchResults,
    WebPage,
}

struct App {
    results: Vec<SearchResult>,
    list_state: ListState,
    mode: Mode,
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
            mode: Mode::Normal,
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
                            self.mode = Mode::Normal;
                        }
                        Err(_) => {
                            self.page_content = vec!["Failed to load page.".to_string()];
                            self.view = View::WebPage;
                            self.mode = Mode::Normal;
                        }
                    }
                }
            }
        }
    }

    fn back_to_results(&mut self) {
        self.view = View::SearchResults;
        self.mode = Mode::Insert;
        self.page_content.clear();
        self.page_scroll = 0;
    }
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

    // Convert HTML to plain text
    let text = html2text::from_read(html.as_bytes(), 100);

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
    let mode_str = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "INSERT",
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", mode_str),
            Style::default()
                .bg(if app.mode == Mode::Insert { Color::Green } else { Color::Blue })
                .fg(Color::White)
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

    // Footer
    let footer_text = match app.mode {
        Mode::Normal => " [i] Insert mode  [q] Quit ",
        Mode::Insert => " [j/k] Navigate  [Enter] Open page  [Esc] Normal mode ",
    };
    let footer = Paragraph::new(footer_text)
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
    let mode_str = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "BROWSE",
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", mode_str),
            Style::default()
                .bg(if app.mode == Mode::Insert { Color::Magenta } else { Color::Blue })
                .fg(Color::White)
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

    // Footer
    let footer_text = match app.mode {
        Mode::Normal => " [i] Browse mode  [Esc][q] Back to results ",
        Mode::Insert => " [j/k] Scroll  [J/K] Scroll 10  [g/G] Top/Bottom  [q] Back ",
    };
    let footer = Paragraph::new(footer_text)
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
                match (&app.view, &app.mode) {
                    // Search Results - Normal Mode
                    (View::SearchResults, Mode::Normal) => match code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('i') => {
                            app.mode = Mode::Insert;
                        }
                        _ => {}
                    },
                    // Search Results - Insert Mode
                    (View::SearchResults, Mode::Insert) => match code {
                        KeyCode::Esc => {
                            app.mode = Mode::Normal;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.previous();
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            app.previous();
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            app.next();
                        }
                        KeyCode::Enter => {
                            app.open_selected();
                        }
                        _ => {}
                    },
                    // Web Page - Normal Mode
                    (View::WebPage, Mode::Normal) => match code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.back_to_results();
                        }
                        KeyCode::Char('i') => {
                            app.mode = Mode::Insert;
                        }
                        _ => {}
                    },
                    // Web Page - Insert (Browse) Mode
                    (View::WebPage, Mode::Insert) => match code {
                        KeyCode::Esc => {
                            app.mode = Mode::Normal;
                        }
                        KeyCode::Char('q') => {
                            app.back_to_results();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.scroll_down(1);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.scroll_up(1);
                        }
                        KeyCode::Char('J') => {
                            app.scroll_down(10);
                        }
                        KeyCode::Char('K') => {
                            app.scroll_up(10);
                        }
                        KeyCode::Char('d') => {
                            app.scroll_down(10);
                        }
                        KeyCode::Char('u') => {
                            app.scroll_up(10);
                        }
                        KeyCode::Char('g') => {
                            app.page_scroll = 0;
                        }
                        KeyCode::Char('G') => {
                            app.page_scroll = app.page_content.len().saturating_sub(10);
                        }
                        KeyCode::PageDown => {
                            app.scroll_down(20);
                        }
                        KeyCode::PageUp => {
                            app.scroll_up(20);
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
                    "  ___  ___  __ _ _ __ ___| |__  ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    " / __|/ _ \\/ _` | '__/ __| '_ \\ ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    " \\__ \\  __/ (_| | | | (__| | | |",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    " |___/\\___|\\__,_|_|  \\___|_| |_|",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled("", Style::default())),
            ];
            let header = Paragraph::new(ascii_art)
                .block(Block::default().borders(Borders::ALL).title("Search"));
            f.render_widget(header, chunks[0]);

            // About content
            let about_content = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Terminal Web Browser",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "  Version 1.0.0",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  A vim-style terminal browser for searching and reading the web.",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled("  FEATURES", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  - Vim-style navigation (hjkl, i for insert, Esc for normal)", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - In-terminal web page rendering", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - Privacy-focused with Brave Search backend", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - No tracking, no cookies, no JavaScript", Style::default().fg(Color::White))),
                Line::from(Span::styled("  - Lightweight and fast (built in Rust)", Style::default().fg(Color::White))),
                Line::from(""),
                Line::from(Span::styled("  WHY SEARCH?", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  Modern browsers are bloated, track everything you do, and pull", Style::default().fg(Color::White))),
                Line::from(Span::styled("  you out of your terminal workflow. Search lets you find and read", Style::default().fg(Color::White))),
                Line::from(Span::styled("  information without leaving the command line.", Style::default().fg(Color::White))),
                Line::from(""),
                Line::from(Span::styled("  KEYBINDINGS", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  i          Enter insert/browse mode", Style::default().fg(Color::White))),
                Line::from(Span::styled("  Esc        Return to normal mode", Style::default().fg(Color::White))),
                Line::from(Span::styled("  j/k        Navigate down/up", Style::default().fg(Color::White))),
                Line::from(Span::styled("  J/K        Scroll 10 lines (in page view)", Style::default().fg(Color::White))),
                Line::from(Span::styled("  g/G        Jump to top/bottom", Style::default().fg(Color::White))),
                Line::from(Span::styled("  Enter      Open selected result", Style::default().fg(Color::White))),
                Line::from(Span::styled("  q          Quit / Go back", Style::default().fg(Color::White))),
                Line::from(""),
                Line::from(Span::styled("  TECHNICAL", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled("  Built with: Rust, ratatui, reqwest, scraper, html2text", Style::default().fg(Color::White))),
                Line::from(Span::styled("  Source: github.com/politikl/search", Style::default().fg(Color::Cyan))),
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
        eprintln!("Usage: search <query>");
        eprintln!("       search about    - Show about information");
        eprintln!("       search history  - Show browsing history");
        eprintln!("Example: search rust programming");
        std::process::exit(1);
    }

    let query = args[1..].join(" ");

    // Check for about command
    if query.to_lowercase() == "about" {
        return show_about();
    }

    // Check for history command
    if query.to_lowercase() == "history" {
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
