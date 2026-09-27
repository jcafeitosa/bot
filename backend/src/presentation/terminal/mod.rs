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
use tokio::sync::{mpsc, watch};

use crate::modules::monitor::views::terminal_dashboard::{
    AppEvent, Dashboard, MonitorSignalLabel, MonitorState, UiCommand,
};
use crate::modules::monitor::{MonitorCommand, MonitorHandle, MonitorSendError};

fn space_command(state: MonitorState) -> UiCommand {
    match state {
        MonitorState::Paused => UiCommand::Resume,
        MonitorState::Running | MonitorState::Resuming => UiCommand::Pause,
    }
}

pub async fn run(
    monitor: MonitorHandle,
    mut events: mpsc::Receiver<AppEvent>,
    mut state_rx: watch::Receiver<MonitorState>,
    mut dashboard_rx: watch::Receiver<Dashboard>,
) -> anyhow::Result<()> {
    let mut app = dashboard_rx.borrow_and_update().clone();
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut event_stream = EventStream::new();
    let result = async {
        loop {
            app.monitor_state = *state_rx.borrow_and_update();
            terminal.draw(|frame| draw(frame, &app))?;
            tokio::select! {
                event = events.recv() => match event {
                    Some(AppEvent::Refresh(_)) => {
                        app = dashboard_rx.borrow().clone();
                        app.monitor_state = *state_rx.borrow();
                    }
                    None => break,
                },
                Ok(()) = state_rx.changed() => {
                    app.monitor_state = *state_rx.borrow_and_update();
                },
                Ok(()) = dashboard_rx.changed() => {
                    app = dashboard_rx.borrow_and_update().clone();
                    app.monitor_state = *state_rx.borrow();
                },
                maybe_event = event_stream.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                send_monitor_command(&monitor, MonitorCommand::Shutdown);
                                break;
                            }
                            KeyCode::Char(' ') => {
                                let ui = space_command(*state_rx.borrow());
                                send_monitor_command(&monitor, ui.into_monitor_command());
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

fn send_monitor_command(handle: &MonitorHandle, command: MonitorCommand) {
    if let Err(MonitorSendError::Full) = handle.send(command) {
        tracing::warn!(target: "ui", "monitor command channel full");
    }
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
    let env_color = if app.header.environment_is_dev {
        Color::Green
    } else {
        Color::Red
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", app.header.environment_label),
            Style::default().fg(env_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " | {} | {} | {} | {} ",
            app.header.run_mode_label,
            app.header.symbol,
            app.header.operation_label,
            app.header.risk_profile_label,
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("RUST BOT · BACKEND / TUI"),
    );
    frame.render_widget(title, chunks[0]);
    let state = match app.monitor_state {
        MonitorState::Running => "RUNNING · OBSERVE ONLY",
        MonitorState::Paused => "PAUSED",
        MonitorState::Resuming => "RESUMING",
    };
    let market = app.market.as_ref();
    let signal = market
        .map(|m| m.signal)
        .unwrap_or(MonitorSignalLabel::Warmup);
    let stats = Paragraph::new(format!(
        "{}   Archive: {}   Last close: {}   SMA({}): {}   SMA({}): {}   Signal: {:?}   Order cap: {:.2} quote{}",
        state,
        app.persistence_status,
        market
            .map(|m| format!("{:.4}", m.close))
            .unwrap_or_else(|| "—".into()),
        app.header.sma_fast,
        market
            .and_then(|m| m.fast_sma)
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "warming up".into()),
        app.header.sma_slow,
        market
            .and_then(|m| m.slow_sma)
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "warming up".into()),
        signal,
        app.max_order_quote,
        if app.header.is_paper {
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

#[cfg(test)]
mod monitor_state_tests {
    use super::*;

    #[test]
    fn space_command_follows_confirmed_monitor_state() {
        assert!(matches!(
            space_command(MonitorState::Running),
            UiCommand::Pause
        ));
        assert!(matches!(
            space_command(MonitorState::Paused),
            UiCommand::Resume
        ));
        assert!(matches!(
            space_command(MonitorState::Resuming),
            UiCommand::Pause
        ));
    }
}
