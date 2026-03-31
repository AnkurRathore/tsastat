mod netlink;
mod protocol;
mod stats;
mod threads;
mod app;
mod ui;

use anyhow::Result;
use crossterm::{event::{self, Event, KeyCode}, execute, terminal::*};
use ratatui::prelude::*;
use std::{env, io, collections::HashMap, time::{Duration, Instant}};
use crate::app::{App, ThreadEntry};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let target_pid: u32 = args.get(1).and_then(|s| s.parse().ok()).expect("PID required");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = App::new();
    let mut client = netlink::TaskstatsClient::new()?;
    let mut prev_stats = HashMap::new();
    let mut last_tick = Instant::now();

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);
        let tids = threads::get_tids(target_pid);
        let mut current_stats = HashMap::new();
        let mut entries = Vec::new();

        for tid in tids {
            if let Ok(stats) = client.get_stats(tid) {
                if let Some(prev) = prev_stats.get(&tid) {
                    let metrics = stats::calculate_deltas(prev, &stats, elapsed);
                    entries.push(ThreadEntry { tid, metrics, raw: stats });
                }
                current_stats.insert(tid, stats);
            }
        }
        app.items = entries;
        prev_stats = current_stats;
        last_tick = now;

        terminal.draw(|f| ui::render(f, &mut app, target_pid))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => app.next(),
                    KeyCode::Up => app.previous(),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}