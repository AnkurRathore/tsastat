mod netlink;
mod protocol;
mod stats;
mod threads;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use netlink::TaskstatsClient;
use protocol::TaskStats;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Row, Table},
};
use std::{any, env, thread, time::Duration};
use std::{
    collections::HashMap,
    io::{self, Stdout},
    time::Instant,
};

fn main() -> anyhow::Result<()> {
    // 1. Get Target PID from CLI args
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: sudo ./target/release/tsastat <PID>");
        std::process::exit(1);
    }
    let target_pid: u32 = args[1].parse().expect("Invalid PID");
    // Initialize Netlink Client
    let mut client = TaskstatsClient::new()?;

    // Setup Terminal UI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the TUI Loop
    let res = run_app(&mut terminal, &mut client, target_pid);

    // Tear down Terminal UI
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }
    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    client: &mut TaskstatsClient,
    pid: u32,
) -> Result<()> {
    let mut prev_stats: HashMap<u32, TaskStats> = HashMap::new();
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_secs(1);

    loop {
        // Gather Data
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);

        let tids = threads::get_tids(pid);
        if tids.is_empty() {
            anyhow::bail!("Process {} has no threads or does not exist.", pid);
        }

        let mut current_stats = HashMap::new();
        let mut display_rows = Vec::new();

        for &tid in &tids {
            // Fetch Data from kernel for this specific thread
            if let Ok(stats) = client.get_stats(pid) {
                current_stats.insert(tid, stats);

                // if we have previous stats, calculate deltas
                if let Some(prev) = prev_stats.get(&tid) {
                    let metrics = stats::calculate_deltas(prev, &stats, elapsed);
                    display_rows.push((tid, metrics));
                }
            }
        }
        // Draw the TUI
        terminal.draw(|f| {
            let size = f.size();

            // Creating the table header
            let header = Row::new(vec![
                "TID",
                "EXEC %",
                "CPU WAIT % (Sched)",
                "I/O WAIT % (Disk)",
                "SWAP WAIT % (RAM)",
            ])
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1);

            // Creating the table rows
            let rows: Vec<Row> = display_rows
                .into_iter()
                .map(|(tid, m)| {
                    // Colorize CPU WAIT: Red if > 20%, Yellow if > 5%, Green otherwise
                    let wait_color = if m.cpu_wait_percent > 20.0 {
                        Color::Red
                    } else if m.cpu_wait_percent > 5.0 {
                        Color::Yellow
                    } else {
                        Color::Green
                    };
                    // Colorize IO WAIT: Red if > 10%
                    let io_color = if m.io_wait_percent > 10.0 {
                        Color::Red
                    } else {
                        Color::White
                    };

                    Row::new(vec![
                        Cell::from(tid.to_string()).style(Style::default().fg(Color::White)),
                        Cell::from(format!("{:.1}%", m.exec_percent))
                            .style(Style::default().fg(Color::Green)),
                        Cell::from(format!("{:.1}%", m.cpu_wait_percent))
                            .style(Style::default().fg(wait_color)),
                        Cell::from(format!("{:.1}%", m.io_wait_percent))
                            .style(Style::default().fg(io_color)),
                        Cell::from(format!("{:.1}%", m.swap_wait_percent))
                            .style(Style::default().fg(Color::DarkGray)),
                    ])
                })
                .collect();
            // Build the Table Widget
            let table = Table::new(
                rows,
                [
                    Constraint::Length(10), // TID
                    Constraint::Length(15), // EXEC
                    Constraint::Length(25), // CPU WAIT
                    Constraint::Length(25), // IO WAIT
                    Constraint::Length(20), // SWAP WAIT
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .title(format!(
                        " TSA-STAT | Target PID: {} | Press 'q' to quit ",
                        pid
                    ))
                    .borders(Borders::ALL),
            );

            f.render_widget(table, size);
        })?;

        // --- 3. STATE UPDATE ---
        prev_stats = current_stats;
        last_tick = now;

        // --- 4. EVENT LOOP (Wait for 'q' or 1 second) ---
        // We poll so the UI stays responsive to keys but updates exactly every 1s
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    return Ok(()); // Quit the app
                }
            }
        }
    }
}
