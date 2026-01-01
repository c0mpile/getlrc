use crate::tui::{
    state::{AppState, Status},
    widgets::{MultiProgress, StatusLegend},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render the TUI (Elm Architecture - View)
pub fn render(frame: &mut Frame, state: &AppState) {
    // Clear the frame to prevent ghost characters
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Reset)),
        frame.area(),
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(5), // Progress
            Constraint::Length(1), // Status Legend
            Constraint::Min(5),    // Logs (responsive)
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_progress(frame, chunks[1], state);
    StatusLegend::render(frame, chunks[2]);
    render_logs(frame, chunks[3], state);
    render_footer(frame, chunks[4], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = match &state.status {
        Status::Idle => "getlrc - Idle",
        Status::Restoring => "getlrc - Restoring Session...",
        Status::Scanning => "getlrc - Scanning...",
        Status::Processing => "getlrc - Processing...",
        Status::Complete => "getlrc - Complete ✓",
        Status::Error(e) => return render_error(frame, area, e),
    };

    let header = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
}

fn render_error(frame: &mut Frame, area: Rect, error: &str) {
    let error_text = format!("Error: {}", error);
    let widget = Paragraph::new(error_text)
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(widget, area);
}

fn render_progress(frame: &mut Frame, area: Rect, state: &AppState) {
    let total = state.total_files + state.skipped;

    let progress = MultiProgress::new(state.downloaded, state.cached, state.skipped, total);

    progress.render(frame, area);
}

fn render_logs(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default().borders(Borders::ALL).title("Logs");
    let inner = block.inner(area);

    // Calculate max width for log entries (account for borders and padding)
    let max_width = inner.width.saturating_sub(2) as usize;

    // Calculate how many lines can fit in the visible area
    let visible_lines = inner.height as usize;

    // Take only the most recent entries that fit in the visible area
    let start_index = state.logs.len().saturating_sub(visible_lines);

    let items: Vec<ListItem> = state
        .logs
        .iter()
        .skip(start_index)
        .map(|log| {
            let truncated = if log.len() > max_width {
                format!("{}...", &log[..max_width.saturating_sub(3)])
            } else {
                log.clone()
            };
            ListItem::new(truncated)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(list, area);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let mut spans = vec![
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit | "),
    ];

    // Add Pause/Resume control based on state
    if state.paused {
        spans.push(Span::styled(
            "r",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" Resume"));
    } else {
        spans.push(Span::styled(
            "p",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" Pause"));
    }

    let footer = Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}
