use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub struct MultiProgress {
    downloaded: usize,
    cached: usize,
    skipped: usize,
    total: usize,
}

impl MultiProgress {
    pub fn new(downloaded: usize, cached: usize, skipped: usize, total: usize) -> Self {
        Self {
            downloaded,
            cached,
            skipped,
            total,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Progress");

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Calculate bar width (leave 2 chars for borders)
        let bar_width = inner.width.saturating_sub(2) as usize;

        if bar_width == 0 || self.total == 0 {
            return;
        }

        // Calculate segment widths
        let downloaded_width = (self.downloaded * bar_width) / self.total.max(1);
        let cached_width = (self.cached * bar_width) / self.total.max(1);
        let skipped_width = (self.skipped * bar_width) / self.total.max(1);

        let filled_width = downloaded_width + cached_width + skipped_width;
        let empty_width = bar_width.saturating_sub(filled_width);

        // Build progress bar spans
        let mut spans = Vec::new();

        if downloaded_width > 0 {
            spans.push(Span::styled(
                "█".repeat(downloaded_width),
                Style::default().fg(Color::Green),
            ));
        }

        if cached_width > 0 {
            spans.push(Span::styled(
                "█".repeat(cached_width),
                Style::default().fg(Color::Yellow),
            ));
        }

        if skipped_width > 0 {
            spans.push(Span::styled(
                "█".repeat(skipped_width),
                Style::default().fg(Color::Blue),
            ));
        }

        if empty_width > 0 {
            spans.push(Span::styled(
                "░".repeat(empty_width),
                Style::default().fg(Color::DarkGray),
            ));
        }

        let bar_line = Line::from(spans);

        // Render bar
        let bar_area = Rect {
            x: inner.x + 1,
            y: inner.y,
            width: bar_width as u16,
            height: 1,
        };

        frame.render_widget(bar_line, bar_area);

        // Render legend
        let legend = Line::from(vec![
            Span::styled("● ", Style::default().fg(Color::Green)),
            Span::raw(format!("Downloaded: {} ", self.downloaded)),
            Span::styled("● ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("Cached: {} ", self.cached)),
            Span::styled("● ", Style::default().fg(Color::Blue)),
            Span::raw(format!("Existing: {}", self.skipped)),
        ]);

        if inner.height > 1 {
            let legend_area = Rect {
                x: inner.x + 1,
                y: inner.y + 1,
                width: inner.width.saturating_sub(2),
                height: 1,
            };
            frame.render_widget(legend, legend_area);
        }
    }
}

pub struct StatusLegend;

impl StatusLegend {
    pub fn render(frame: &mut Frame, area: Rect) {
        let legend = Line::from(vec![
            Span::styled("[✓]", Style::default().fg(Color::Green)),
            Span::raw(" Downloaded | "),
            Span::styled("[~]", Style::default().fg(Color::Yellow)),
            Span::raw(" Cached | "),
            Span::styled("[○]", Style::default().fg(Color::Blue)),
            Span::raw(" Existing | "),
            Span::styled("[✗]", Style::default().fg(Color::Red)),
            Span::raw(" Not Found | "),
            Span::styled("[!]", Style::default().fg(Color::Magenta)),
            Span::raw(" Error"),
        ]);

        frame.render_widget(legend, area);
    }
}
