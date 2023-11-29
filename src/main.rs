extern crate core;

mod meter;

use std::{io, thread};
use std::time::Duration;
use std::time::Instant;

use tui::{backend::{CrosstermBackend, Backend}, widgets::{Widget, Block, Borders, Gauge}, layout::{Layout, Constraint, Direction}, Frame, Terminal, style::{Color, Style}, symbols};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use tui::style::Modifier;
use tui::text::Span;
use tui::widgets::{Axis, Chart, Dataset, GraphType};
// use crossterm::style::Color;
use crate::meter::{Account, Client, Reader, Update};

struct App {
    account: Account,
    day: u16,
}
impl App {
    fn new() -> App {
        return App {
            account: Account::new(vec![Client::new(), Client::new()]),
            day: 0,
        }
    }
    /// Call add_point on each of the meters.
    /// Call add_point on master meter.
    fn on_tick(&mut self) {
        if self.day == 900 {
            return
        }
        if self.day == 0 {
            // Add initial points.
            for _ in 0..30 {
                for idx in 0..self.account.master_meter.clients.len() {
                    self.account.master_meter.clients[idx].meter.add_point();
                    self.account.master_meter.clients[idx].c_update_totals();
                }
                self.account.master_meter.add_point();
                self.account.master_meter.m_update_totals();
            }

        }
        for idx in 0..self.account.master_meter.clients.len() {
            self.account.master_meter.clients[idx].meter.add_point();
            self.account.master_meter.clients[idx].c_update_totals();
        }
        self.account.master_meter.add_point();
        self.account.master_meter.m_update_totals();
        // self.account.master_meter.update_totals(
        //     &self.account.master_meter.meter,
        // )
        self.day += 1;
        self.account.master_meter.clients[0].get_data();
        self.account.master_meter.clients[1].get_data();
    }
}


fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let datasets = vec![
        Dataset::default()
            .name("data2")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&app.account.master_meter.clients[0]._data_array),
        Dataset::default()
            .name("data3")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .data(&app.account.master_meter.clients[1]._data_array),
    ];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(50),
        ].as_ref())
        .split(f.size());
    let gauge = Gauge::default()
        .block(Block::default().title("C1").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Yellow))
        .percent(
                match app.account.master_meter.clients[0].meter.history.len() > 0 {
                true => {
                    app.account.master_meter.clients[0].meter.history[
                            app.account.master_meter.clients[0].meter.history.len()-1
                        ].1 as u16
                },
                false => {
                    0 as u16
                },
            }
        );
    f.render_widget(gauge, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default().title(app.account.master_meter.clients[1].meter.history.len().to_string()).borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Yellow))
        .percent(
            match (app.account.master_meter.clients[1].meter.history.len() > 0) {
                true => {
                    app.account.master_meter.clients[1].meter.history[
                            app.account.master_meter.clients[1].meter.history.len()-1
                        ].1 as u16
                },
                false => {
                    0 as u16
                },
            }
        );
    f.render_widget(gauge, chunks[1]);

    let chart = Chart::new(datasets)
                .block(
            Block::default()
                .title(Span::styled(
                    "Chart 1",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("X Axis")
                .style(Style::default().fg(Color::Gray))
                // .labels(x_labels)
                .bounds([0.0, 30.0]),
        )
        .y_axis(
            Axis::default()
                .title("Y Axis")
                .style(Style::default().fg(Color::Gray))
                .labels(vec![
                    Span::styled("-20", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("0"),
                    Span::styled("20", Style::default().add_modifier(Modifier::BOLD)),
                ])
                .bounds([0.0, 30.0]),
        );
    f.render_widget(chart, chunks[2]);
    // let block = Block::default()
    //     .title("Block")
    //     .borders(Borders::ALL);
    // f.render_widget(block, chunks[0]);
    // let block = Block::default()
    //     .title("Block 2")
    //     .borders(Borders::ALL);
    // f.render_widget(block, chunks[1]);
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &app))?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn main() -> Result<(), io::Error> {
    println!("Starting...");
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    // let tick_rate = Duration::from_millis(300);
    let tick_rate = Duration::from_millis(120);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);


    // terminal.draw(|f| {
    //     let size=f.size();
    //     let block = Block::default()
    //         .title("Block")
    //         .borders(Borders::ALL);
    //     f.render_widget(block, size);
    // })?;
    // thread::sleep(Duration::from_millis(1000));
    // terminal.draw(ui);
    // thread::sleep(Duration::from_millis(1000));

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;
    Ok(())
}
