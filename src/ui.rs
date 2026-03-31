use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Row, Table, Paragraph},

};
use crate::app::App;

pub fn render(f: &mut Frame, app: &mut App, pid: u32) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(f.size());

    let header = Row::new(vec!["TID", "EXEC %", "CPU WAIT %", "I/O WAIT %", "SWAP WAIT %"])
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app.items.iter().map(|item| {
        Row::new(vec![
            item.tid.to_string(),
            format!("{:.1}%", item.metrics.exec_percent),
            format!("{:.1}%", item.metrics.cpu_wait_percent),
            format!("{:.1}%", item.metrics.io_wait_percent),
            format!("{:.1}%", item.metrics.swap_wait_percent),
        ])
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(10),
        Constraint::Length(15),
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Length(20),
    ])
    .header(header)
    .highlight_symbol(">> ")
    .highlight_style(Style::default().bg(Color::Indexed(236)).add_modifier(Modifier::BOLD))
    .block(Block::default().borders(Borders::ALL).title(format!(" TSA-STAT | Target PID: {} ", pid)));

    f.render_stateful_widget(table, chunks[0], &mut app.table_state);

    // Inspector Pane
    if let Some(i) = app.table_state.selected() {
        if let Some(item) = app.items.get(i) {
            let details = Paragraph::new(vec![
                Line::from(vec![Span::styled(format!(" Thread {} Raw Details ", item.tid), Style::default().fg(Color::Yellow))]),
                Line::from(format!("  ABI Version:      {}", item.raw.version)),
                Line::from(format!("  CPU Execution:    {} ns", item.raw.cpu_run_real_total)),
                Line::from(format!("  Scheduler Delay:  {} ns", item.raw.cpu_delay_total)),
                Line::from(format!("  Exit Code:        {}", item.raw.ac_exitcode)),
            ])
            .block(Block::default().borders(Borders::ALL).title(" Kernel Taskstats "));
            f.render_widget(details, chunks[1]);
        }
    }
}