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
mod bt;
mod maestro_client;
mod ui;

use app::App;
use maestro_client::{ClientCommand, ClientEvent};

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Create channels
    let (tx_event, rx_event) = mpsc::unbounded_channel();
    let (tx_cmd, rx_cmd) = mpsc::unbounded_channel();

    // Spawn client in a separate blocking task to isolate it completely
    tokio::spawn(maestro_client::run_loop(tx_event, rx_cmd));

    // Initial check
    tx_cmd.send(ClientCommand::CheckConnection).ok();

    let res = run_app(&mut terminal, &mut app, tx_cmd, rx_event).await;

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
    mut rx_event: mpsc::UnboundedReceiver<ClientEvent>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(50);

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Poll for keyboard events with timeout - this is non-blocking
        if event::poll(tick_rate)? && let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                KeyCode::Tab => {
                    app.next_tab();
                }
                KeyCode::Char('c') => {
                    tx_cmd.send(ClientCommand::CheckConnection).ok();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.selected_tab == 1 {
                        app.next_setting();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.selected_tab == 1 {
                        app.previous_setting();
                    }
                }
                KeyCode::Left => {
                    if app.selected_tab == 1 {
                        handle_numeric_change(app, &tx_cmd, -1.0);
                    }
                }
                KeyCode::Right => {
                    if app.selected_tab == 1 {
                        handle_numeric_change(app, &tx_cmd, 1.0);
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

        // Process all pending client events (non-blocking)
        while let Ok(event) = rx_event.try_recv() {
            process_client_event(app, &tx_cmd, event)?;
        }

        app.on_tick();

        if app.should_quit {
            return Ok(());
        }
    }
}

fn process_client_event(
    app: &mut App,
    tx_cmd: &mpsc::UnboundedSender<ClientCommand>,
    event: ClientEvent,
) -> Result<()> {
    match event {
        ClientEvent::ConnectionState(state) => {
            app.connection_state = state.clone();
            if matches!(state, maestro_client::ConnectionState::Connected) {
                tx_cmd.send(ClientCommand::GetSoftware)?;
                tx_cmd.send(ClientCommand::GetHardware)?;

                let mut fetched_keys = std::collections::HashSet::new();
                for item in &app.settings {
                    if !fetched_keys.contains(&item.key) {
                        tx_cmd.send(ClientCommand::GetSetting(item.key.clone()))?;
                        fetched_keys.insert(item.key.clone());
                    }
                }
                tx_cmd.send(ClientCommand::GetSetting("gesture-control".to_string()))?;
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
            app.battery = info.battery;
        }
        ClientEvent::Setting(key, val) => {
            if key == "gesture-control" {
                app.gesture_control = val;
            } else if key == "eq" {
                let trimmed = val.trim_matches(|c| c == '[' || c == ']');
                let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim()).collect();

                if parts.len() == 5 {
                    for (i, part) in parts.iter().enumerate() {
                        if part.parse::<f32>().is_ok()
                            && let Some(item) = app.settings.iter_mut()
                                .find(|it| it.key == "eq" && it.index == Some(i))
                        {
                            item.value = part.to_string();
                        }
                    }
                }
            } else {
                app.update_setting(key, val);
            }
        }
        ClientEvent::Error(msg) => {
            app.set_error(msg);
        }
    }
    Ok(())
}

fn handle_numeric_change(app: &mut App, tx_cmd: &mpsc::UnboundedSender<ClientCommand>, direction: f32) {
    if let Some(idx) = app.settings_state.selected() {
        let item = &mut app.settings[idx];
        
        if let Some((min, max, step)) = item.range {
            let change = direction * step;
            
            if item.key == "balance" {
                // Parse current balance
                let current_val = if item.value.contains("left:") {
                    let parts: Vec<&str> = item.value.split(',').collect();
                    let mut l = 100;
                    let mut r = 100;
                    
                    for part in parts {
                        if let Some(v) = part.split(':').nth(1) {
                            let n = v.trim().trim_end_matches('%').parse::<i32>().unwrap_or(100);
                            if part.contains("left") { l = n; }
                            if part.contains("right") { r = n; }
                        }
                    }
                    
                    if r == 100 {
                        100 - l
                    } else {
                        r - 100
                    }
                } else {
                    0
                };
                
                let new_val = (current_val as f32 + change).clamp(min, max) as i32;
                
                let l = (100 - new_val).min(100);
                let r = (100 + new_val).min(100);
                item.value = format!("left: {}%, right: {}%", l, r);
                
                let _ = tx_cmd.send(ClientCommand::SetSetting(item.key.clone(), new_val.to_string()));
                
            } else if item.key == "eq" {
                let current_val = item.value.parse::<f32>().unwrap_or(0.0);
                let new_val = (current_val + change).clamp(min, max);
                
                item.value = format!("{:.2}", new_val);
            }
        }
    }
    
    // Split logic to avoid borrow checker issues for EQ
    if let Some(idx) = app.settings_state.selected() {
        let key = app.settings[idx].key.clone();
        if key == "eq" {
             let mut eq_values = [0.0f32; 5];
             for it in &app.settings {
                 if it.key == "eq"
                     && let Some(i) = it.index
                     && i < 5 {
                         eq_values[i] = it.value.parse::<f32>().unwrap_or(0.0);
                 }
             }
             
             let args = eq_values.iter().map(|v| format!("{:.2}", v)).collect::<Vec<_>>().join(" ");
             let _ = tx_cmd.send(ClientCommand::SetSetting("eq".to_string(), args));
        }
    }
}

fn handle_setting_change(app: &App, tx_cmd: &mpsc::UnboundedSender<ClientCommand>) {
    if let Some(idx) = app.settings_state.selected() {
        let item = &app.settings[idx];
        
        if item.options.is_empty() {
            return;
        }
        
        let current_val = item.value.to_lowercase();
        let current_opt_idx = item.options.iter().position(|o| o.to_lowercase() == current_val);
        
        let next_val = if let Some(i) = current_opt_idx {
            item.options[(i + 1) % item.options.len()].clone()
        } else if !item.options.is_empty() {
            item.options[0].clone()
        } else {
            return;
        };
        
        let _ = tx_cmd.send(ClientCommand::SetSetting(item.key.clone(), next_val));
    }
}
