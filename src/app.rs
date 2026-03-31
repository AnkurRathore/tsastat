use crate::protocol::TaskStats;
use crate::stats::ThreadMetrics;
use ratatui::widgets::TableState;

/// Data representating a single thread in the UI
pub struct ThreadEntry {
    pub tid: u32,
    pub metrics: ThreadMetrics,
    pub raw: TaskStats,
}

pub struct App {
    pub table_state: TableState,
    pub items: Vec<ThreadEntry>,
}

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0)); // Start with the first row selected
        Self {
            table_state,
            items: Vec::new(),
        }
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
}
