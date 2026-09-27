#![allow(dead_code)]
use crate::modules::monitor::views::presentation_contract::MonitorCommand;

#[derive(Debug, Clone)]
pub enum UiCommand {
    Pause,
    Resume,
    Quit,
}

impl UiCommand {
    pub fn into_monitor_command(self) -> MonitorCommand {
        match self {
            UiCommand::Pause => MonitorCommand::Pause,
            UiCommand::Resume => MonitorCommand::Resume,
            UiCommand::Quit => MonitorCommand::Shutdown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorState {
    Running,
    Paused,
    Resuming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorSignalLabel {
    Warmup,
    Hold,
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct TerminalMarketRow {
    pub close: f64,
    pub fast_sma: Option<f64>,
    pub slow_sma: Option<f64>,
    pub signal: MonitorSignalLabel,
    pub candle_timestamp_ms: i64,
}

#[derive(Debug, Clone)]
pub struct DashboardHeader {
    pub environment_label: String,
    pub environment_is_dev: bool,
    pub run_mode_label: String,
    pub is_paper: bool,
    pub symbol: String,
    pub operation_label: String,
    pub risk_profile_label: String,
    pub sma_fast: usize,
    pub sma_slow: usize,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Refresh(Dashboard),
}

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub header: DashboardHeader,
    pub max_order_quote: f64,
    pub market: Option<TerminalMarketRow>,
    pub advisor_note: Option<String>,
    pub last_error: Option<String>,
    pub logs: Vec<String>,
    pub monitor_state: MonitorState,
    pub persistence_status: String,
    pub paper_open_positions: usize,
}

impl Dashboard {
    pub fn new(header: DashboardHeader, max_order_quote: f64) -> Self {
        Self {
            header,
            max_order_quote,
            market: None,
            advisor_note: None,
            last_error: None,
            logs: Vec::new(),
            monitor_state: MonitorState::Running,
            persistence_status: "OFF".into(),
            paper_open_positions: 0,
        }
    }
    pub fn set_paper_open_positions(&mut self, open_positions: usize) {
        self.paper_open_positions = open_positions;
    }
    pub fn update_market(&mut self, market: TerminalMarketRow, note: Option<String>) {
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
