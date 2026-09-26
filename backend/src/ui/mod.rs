use std::time::Duration;

use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame, Terminal,
};
use tokio::sync::mpsc;

use crate::{
    config::{Config, Environment, RunMode},
    risk::RiskLimits,
    strategy::{Signal, StrategySnapshot},
};

#[derive(Debug, Clone)]
pub enum UiCommand {
    Pause,
    Resume,
    Quit,
}
#[derive(Debug, Clone)]
pub enum AppEvent {
    Refresh(Dashboard),
}

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub config: Config,
    pub limits: RiskLimits,
    pub market: Option<StrategySnapshot>,
    pub advisor_note: Option<String>,
    pub last_error: Option<String>,
    pub logs: Vec<String>,
    pub paused: bool,
    pub paper_open_positions: usize,
}

impl Dashboard {
    pub fn new(config: Config, limits: RiskLimits) -> Self {
        Self {
            config,
            limits,
            market: None,
            advisor_note: None,
            last_error: None,
            logs: Vec::new(),
            paused: false,
            paper_open_positions: 0,
        }
    }
    pub fn set_paper_open_positions(&mut self, open_positions: usize) {
        self.paper_open_positions = open_positions;
    }
    pub fn update_market(&mut self, market: StrategySnapshot, note: Option<String>) {
        self.market = Some(market);
        self.advisor_note = note;
        self.last_error = None;
    }
    pub fn set_error(&mut self, message: String) {
        self.last_error = Some(message);
    }
    pub fn push_log(&mut self, message: String) {
        self.logs.push(message);
        if self.logs.len() > 10 {
            self.logs.remove(0);
        }
    }
}

pub async fn run(
    mut app: Dashboard,
    commands: mpsc::Sender<UiCommand>,
    mut events: mpsc::Receiver<AppEvent>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut event_stream = EventStream::new();
    let result = async {
        loop {
            terminal.draw(|frame| draw(frame, &app))?;
            tokio::select! {
                Some(AppEvent::Refresh(next)) = events.recv() => { app = next; },
                maybe_event = event_stream.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => { let _ = commands.send(UiCommand::Quit).await; break; }
                            KeyCode::Char(' ') => {
                                app.paused = !app.paused;
                                let cmd = if app.paused { UiCommand::Pause } else { UiCommand::Resume };
                                let _ = commands.send(cmd).await;
                            }
                            _ => {}
                        },
                        Some(Err(e)) => return Err(anyhow::anyhow!(e)),
                        _ => {}
                    }
                },
                _ = tokio::time::sleep(Duration::from_millis(250)) => {},
            }
        }
        anyhow::Ok(())
    }.await;
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn draw(frame: &mut Frame, app: &Dashboard) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(5),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(frame.area());
    let env_color = if app.config.environment == Environment::Dev {
        Color::Green
    } else {
        Color::Red
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", app.config.environment),
            Style::default().fg(env_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " | {:?} | {} | {} | {} ",
            app.config.run_mode,
            app.config.market.symbol,
            app.config.operation,
            app.config.risk_profile
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("RUST BOT · BACKEND / TUI"),
    );
    frame.render_widget(title, chunks[0]);
    let state = if app.paused {
        "PAUSED"
    } else {
        "RUNNING · OBSERVE ONLY"
    };
    let market = app.market.as_ref();
    let stats = Paragraph::new(format!(
        "{}   Last close: {}   SMA({}): {}   SMA({}): {}   Signal: {:?}   Order cap: {:.2} quote{}",
        state,
        market
            .map(|m| format!("{:.4}", m.close))
            .unwrap_or_else(|| "—".into()),
        app.config.strategy.sma_fast,
        market
            .and_then(|m| m.fast_sma)
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "warming up".into()),
        app.config.strategy.sma_slow,
        market
            .and_then(|m| m.slow_sma)
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "warming up".into()),
        market.map(|m| m.signal).unwrap_or(Signal::Warmup),
        app.limits.max_order_quote,
        if app.config.run_mode == RunMode::Paper {
            format!("   Paper positions: {}", app.paper_open_positions)
        } else {
            String::new()
        }
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Market · Strategy · Deterministic risk limits"),
    );
    frame.render_widget(stats, chunks[1]);
    let rows = app
        .logs
        .iter()
        .rev()
        .map(|line| Row::new(vec![Cell::from(line.as_str())]));
    let logs = Table::new(rows, [Constraint::Percentage(100)])
        .header(
            Row::new(vec!["Recent events"]).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Categorized activity"),
        );
    frame.render_widget(logs, chunks[2]);
    let advisory = app
        .advisor_note
        .as_deref()
        .unwrap_or("Jev disabled or no advisory result yet");
    let message = app.last_error.as_deref().unwrap_or(advisory);
    let note = Paragraph::new(message)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(if app.last_error.is_some() {
                    "Last error"
                } else {
                    "Jev advisory (never authorizes orders)"
                }),
        );
    frame.render_widget(note, chunks[3]);
    let footer = Paragraph::new(
        "[Space] pause/resume   [q/Esc] quit   Orders: disabled in v1   Production: blocked",
    )
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[4]);
}
