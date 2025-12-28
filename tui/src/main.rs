use std::{io, time::Duration};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

mod app;
mod cli_client;
mod ui;

use app::App;
use cli_client::{ClientCommand, ClientEvent};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Create channels
    let (tx_event, mut rx_event) = mpsc::unbounded_channel();
    let (tx_cmd, rx_cmd) = mpsc::unbounded_channel();

    // Spawn CLI client
    tokio::spawn(cli_client::run_loop(tx_event, rx_cmd));

    // Initial check
    tx_cmd.send(ClientCommand::CheckConnection).ok();

    let res = run_app(&mut terminal, &mut app, tx_cmd, &mut rx_event).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tx_cmd: mpsc::UnboundedSender<ClientCommand>,
    rx_event: &mut mpsc::UnboundedReceiver<ClientEvent>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let poll_rate = Duration::from_secs(5); // Poll status/settings every 5s
    let mut last_tick = std::time::Instant::now();
    let mut last_poll = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => {
                        app.next_tab();
                    }
                    KeyCode::Char('c') => {
                        tx_cmd.send(ClientCommand::CheckConnection)?;
                    }
                    KeyCode::Down => {
                         if app.selected_tab == 1 {
                             app.next_setting();
                         }
                    }
                    KeyCode::Up => {
                        if app.selected_tab == 1 {
                            app.previous_setting();
                        }
                    }
                    KeyCode::Enter => {
                        if app.selected_tab == 1 {
                            handle_setting_change(app, &tx_cmd);
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Handle events
        while let Ok(event) = rx_event.try_recv() {
            match event {
                ClientEvent::ConnectionState(state) => {
                    app.connection_state = state.clone();
                    if matches!(state, cli_client::ConnectionState::Connected) {
                        // Refresh all
                        // We use GetRuntime instead of GetBattery as it provides more info
                        tx_cmd.send(ClientCommand::GetRuntime)?;
                        tx_cmd.send(ClientCommand::GetSoftware)?;
                        tx_cmd.send(ClientCommand::GetHardware)?;
                        for item in &app.settings {
                             tx_cmd.send(ClientCommand::GetSetting(item.key.clone()))?;
                        }
                    }
                }
                ClientEvent::Software(info) => {
                    app.software = info;
                }
                ClientEvent::Hardware(info) => {
                    app.hardware = info;
                }
                ClientEvent::Runtime(info) => {
                    app.runtime = info.clone();
                    // Runtime also contains battery info, let's update that too
                    app.battery = info.battery;
                }
                ClientEvent::Setting(key, val) => {
                    app.update_setting(key, val);
                }
                ClientEvent::Error(msg) => {
                    eprintln!("Error: {}", msg);
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = std::time::Instant::now();
        }

        if last_poll.elapsed() >= poll_rate {
             if matches!(app.connection_state, cli_client::ConnectionState::Connected) {
                 tx_cmd.send(ClientCommand::GetRuntime)?;
             }
             last_poll = std::time::Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_setting_change(app: &App, tx_cmd: &mpsc::UnboundedSender<ClientCommand>) {
    if let Some(idx) = app.settings_state.selected() {
        let item = &app.settings[idx];
        
        if item.options.is_empty() {
            return; // Read-only setting
        }
        
        // Cycle to next option
        let current_val = item.value.to_lowercase();
        // Find current index in options
        let current_opt_idx = item.options.iter().position(|o| o.to_lowercase() == current_val);
        
        let next_val = if let Some(i) = current_opt_idx {
            item.options[(i + 1) % item.options.len()].clone()
        } else if !item.options.is_empty() {
            item.options[0].clone()
        } else {
            return;
        };
        
        // Send set command
        let _ = tx_cmd.send(ClientCommand::SetSetting(item.key.clone(), next_val));
    }
}