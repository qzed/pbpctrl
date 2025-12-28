use tokio::process::Command;
use tokio::sync::mpsc;
use anyhow::Result;
use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connected,
}

#[derive(Debug, Clone, Default)]
pub struct BatteryState {
    pub case_level: Option<u8>,
    pub case_status: String,
    pub left_level: Option<u8>,
    pub left_status: String,
    pub right_level: Option<u8>,
    pub right_status: String,
}

#[derive(Debug, Clone, Default)]
pub struct SoftwareInfo {
    pub case_version: String,
    pub left_version: String,
    pub right_version: String,
}

#[derive(Debug, Clone, Default)]
pub struct HardwareInfo {
    pub case_serial: String,
    pub left_serial: String,
    pub right_serial: String,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeInfo {
    pub battery: BatteryState,
    pub placement_left: String,
    pub placement_right: String,
    pub peer_local: String,
    pub peer_remote: String,
}

#[derive(Debug, Clone)]
pub enum ClientEvent {
    ConnectionState(ConnectionState),
    Software(SoftwareInfo),
    Hardware(HardwareInfo),
    Runtime(RuntimeInfo),
    Setting(String, String), // key, value
    Error(String), 
}

#[derive(Debug, Clone)]
pub enum ClientCommand {
    CheckConnection,
    GetSoftware,
    GetHardware,
    GetRuntime,
    GetSetting(String),
    SetSetting(String, String),
}

pub async fn run_loop(
    tx: mpsc::UnboundedSender<ClientEvent>,
    mut rx: mpsc::UnboundedReceiver<ClientCommand>,
) {
    let binary = "pbpctrl"; 
    
    // Check if we can run it
    let _ = Command::new(binary).arg("--help").output().await;

    loop {
        match rx.recv().await {
            Some(ClientCommand::CheckConnection) => {
                match run_cmd(binary, &["show", "software"]).await {
                    Ok(_) => {
                        let _ = tx.send(ClientEvent::ConnectionState(ConnectionState::Connected));
                    }
                    Err(_) => {
                         let _ = tx.send(ClientEvent::ConnectionState(ConnectionState::Disconnected));
                    }
                }
            }
            Some(ClientCommand::GetSoftware) => {
                match run_cmd(binary, &["show", "software"]).await {
                    Ok(output) => {
                        let info = parse_software(&output);
                        let _ = tx.send(ClientEvent::Software(info));
                    }
                    Err(e) => {
                         let _ = tx.send(ClientEvent::Error(format!("Software info error: {}", e)));
                    }
                }
            }
            Some(ClientCommand::GetHardware) => {
                match run_cmd(binary, &["show", "hardware"]).await {
                    Ok(output) => {
                        let info = parse_hardware(&output);
                        let _ = tx.send(ClientEvent::Hardware(info));
                    }
                    Err(e) => {
                         let _ = tx.send(ClientEvent::Error(format!("Hardware info error: {}", e)));
                    }
                }
            }
            Some(ClientCommand::GetRuntime) => {
                match run_cmd(binary, &["show", "runtime"]).await {
                    Ok(output) => {
                         let info = parse_runtime(&output);
                         let _ = tx.send(ClientEvent::Runtime(info));
                    }
                    Err(e) => {
                         let _ = tx.send(ClientEvent::Error(format!("Runtime info error: {}", e)));
                    }
                }
            }
            Some(ClientCommand::GetSetting(key)) => {
                match run_cmd(binary, &["get", &key]).await {
                    Ok(output) => {
                        let val = output.trim().to_string();
                        let _ = tx.send(ClientEvent::Setting(key, val));
                    }
                    Err(e) => {
                        let _ = tx.send(ClientEvent::Error(format!("Get {} error: {}", key, e)));
                    }
                }
            }
            Some(ClientCommand::SetSetting(key, val)) => {
                let mut args = vec!["set", &key];
                let val_parts: Vec<&str> = val.split_whitespace().collect();
                args.extend(val_parts);

                match run_cmd(binary, &args).await {
                    Ok(_) => {
                         if let Ok(output) = run_cmd(binary, &["get", &key]).await {
                             let v = output.trim().to_string();
                             let _ = tx.send(ClientEvent::Setting(key, v));
                         }
                    }
                    Err(e) => {
                        let _ = tx.send(ClientEvent::Error(format!("Set {} error: {}", key, e)));
                    }
                }
            }
            None => break,
        }
    }
}

async fn run_cmd(binary: &str, args: &[&str]) -> Result<String> {
    let mut final_cmd = binary.to_string();
    
    if std::path::Path::new("./pbpctrl").exists() {
        final_cmd = "./pbpctrl".to_string();
    }

    let output = Command::new(&final_cmd)
        .args(args)
        .kill_on_drop(true)
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        anyhow::bail!("Command failed: {}", err.trim())
    }
}

fn parse_runtime(output: &str) -> RuntimeInfo {
    let mut info = RuntimeInfo::default();
    let mut current_section = "";
    
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        
        if line.ends_with(':') {
            current_section = line.trim_end_matches(':');
            continue;
        }
        
        // Split "key: value"
        let (key, val) = match line.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match current_section {
            "battery" => {
                // Format: "100% (not charging)" or "unknown"
                let (level_str, status_str) = if let Some(idx) = val.find('(') {
                    let (l, r) = val.split_at(idx);
                    (l.trim(), r.trim_matches(|c| c == '(' || c == ')'))
                } else {
                    (val, "unknown")
                };
                
                let level = if level_str == "unknown" {
                    None
                } else {
                    level_str.trim_end_matches('%').parse::<u8>().ok()
                };
                
                match key {
                    "case" => {
                        info.battery.case_level = level;
                        info.battery.case_status = status_str.to_string();
                    }
                    "left bud" => {
                        info.battery.left_level = level;
                        info.battery.left_status = status_str.to_string();
                    }
                    "right bud" => {
                        info.battery.right_level = level;
                        info.battery.right_status = status_str.to_string();
                    }
                    _ => {}
                }
            }
            "placement" => {
                match key {
                    "left bud" => info.placement_left = val.to_string(),
                    "right bud" => info.placement_right = val.to_string(),
                    _ => {}
                }
            }
            "connection" => {
                 match key {
                     "local" => info.peer_local = val.to_string(),
                     "remote" => info.peer_remote = val.to_string(),
                     _ => {}
                 }
            }
            _ => {}
        }
    }

    info
}

fn parse_software(output: &str) -> SoftwareInfo {
    let mut info = SoftwareInfo::default();
    
    let re = Regex::new(r"(?m)^\s*(case|left bud|right bud):\s+([^\s]+)").unwrap();
    
    for cap in re.captures_iter(output) {
        let name = cap.get(1).unwrap().as_str();
        let version = cap.get(2).unwrap().as_str().to_string();
        
        match name {
            "case" => info.case_version = version,
            "left bud" => info.left_version = version,
            "right bud" => info.right_version = version,
            _ => {}
        }
    }
    info
}

fn parse_hardware(output: &str) -> HardwareInfo {
    let mut info = HardwareInfo::default();
    
    let re = Regex::new(r"(?m)^\s*(case|left bud|right bud):\s+([^\s]+)").unwrap();
    
    for cap in re.captures_iter(output) {
        let name = cap.get(1).unwrap().as_str();
        let serial = cap.get(2).unwrap().as_str().to_string();
        
        match name {
            "case" => info.case_serial = serial,
            "left bud" => info.left_serial = serial,
            "right bud" => info.right_serial = serial,
            _ => {}
        }
    }
    info
}
