use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
struct RawPortEntry {
    pid: i32,
    process_name: String,
    user: String,
    protocol: String,
    host: String,
    port: u16,
    state: String,
    ip_version: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PortProcess {
    pid: i32,
    process_name: String,
    user: String,
    protocol: String,
    port: u16,
    state: String,
    hosts: Vec<String>,
    host_summary: String,
    ip_versions: Vec<String>,
    command: String,
    cwd: Option<String>,
    started_at: Option<String>,
    started_at_ts: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortListResponse {
    current_user: String,
    items: Vec<PortProcess>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KillResult {
    pid: i32,
    signal: String,
    success: bool,
    message: String,
}

#[derive(Debug, Clone)]
struct ProcessContext {
    command: String,
    cwd: Option<String>,
    started_at: Option<String>,
    started_at_ts: Option<i64>,
}

#[tauri::command]
fn list_ports() -> Result<PortListResponse, String> {
    let raw_entries = list_raw_ports()?;
    let contexts = load_process_contexts(&raw_entries);
    let mut grouped: HashMap<String, PortProcess> = HashMap::new();

    for entry in raw_entries {
        let key = format!("{}:{}:{}", entry.pid, entry.protocol, entry.port);
        let context = contexts.get(&entry.pid).cloned().unwrap_or(ProcessContext {
            command: entry.process_name.clone(),
            cwd: None,
            started_at: None,
            started_at_ts: None,
        });

        let item = grouped.entry(key).or_insert_with(|| PortProcess {
            pid: entry.pid,
            process_name: entry.process_name.clone(),
            user: entry.user.clone(),
            protocol: entry.protocol.clone(),
            port: entry.port,
            state: entry.state.clone(),
            hosts: Vec::new(),
            host_summary: String::new(),
            ip_versions: Vec::new(),
            command: context.command.clone(),
            cwd: context.cwd.clone(),
            started_at: context.started_at.clone(),
            started_at_ts: context.started_at_ts,
        });

        if !item.hosts.iter().any(|host| host == &entry.host) {
            item.hosts.push(entry.host.clone());
        }

        if !item.ip_versions.iter().any(|ip| ip == &entry.ip_version) {
            item.ip_versions.push(entry.ip_version.clone());
        }
    }

    let mut items = grouped
        .into_values()
        .map(|mut item| {
            item.hosts.sort();
            item.ip_versions.sort();
            item.host_summary = summarize_hosts(&item.hosts, &item.ip_versions);
            item
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .started_at_ts
            .cmp(&left.started_at_ts)
            .then(left.port.cmp(&right.port))
            .then(left.process_name.cmp(&right.process_name))
            .then(left.pid.cmp(&right.pid))
    });

    Ok(PortListResponse {
        current_user: current_username(),
        items,
    })
}

#[tauri::command]
fn kill_process(pid: i32, force: Option<bool>) -> Result<KillResult, String> {
    if pid <= 0 {
        return Err("无效的 PID".to_string());
    }

    let force = force.unwrap_or(false);
    let signal = if force { "-KILL" } else { "-TERM" };
    let output = Command::new("/bin/kill")
        .args([signal, &pid.to_string()])
        .output()
        .map_err(|error| format!("failed to execute kill: {error}"))?;

    if output.status.success() {
        for _ in 0..10 {
            if !process_exists(pid) {
                return Ok(KillResult {
                    pid,
                    signal: signal.trim_start_matches('-').to_string(),
                    success: true,
                    message: "已结束".to_string(),
                });
            }

            thread::sleep(Duration::from_millis(120));
        }

        return Err(if force {
            format!("进程 {pid} 在强制结束后仍未退出")
        } else {
            format!("进程 {pid} 未响应结束信号，请尝试强制结束")
        });
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(normalize_kill_error(pid, &stderr))
}

fn list_raw_ports() -> Result<Vec<RawPortEntry>, String> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcLnPTu"])
        .output()
        .map_err(|error| format!("failed to execute lsof: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "lsof returned a non-zero status".to_string()
        } else {
            stderr
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pid: Option<i32> = None;
    let mut process_name = String::new();
    let mut user = String::new();
    let mut protocol = String::new();
    let mut state = String::new();
    let mut entries = Vec::new();
    let mut seen = HashSet::new();

    for raw_line in stdout.lines() {
        if raw_line.is_empty() {
            continue;
        }

        let mut chars = raw_line.chars();
        let Some(prefix) = chars.next() else {
            continue;
        };
        let value = chars.as_str().trim();

        match prefix {
            'p' => {
                pid = value.parse::<i32>().ok();
                process_name.clear();
                user.clear();
                protocol.clear();
                state.clear();
            }
            'c' => process_name = value.to_string(),
            'L' => user = value.to_string(),
            'P' => protocol = value.to_string(),
            'T' => {
                if let Some(found_state) = value.strip_prefix("ST=") {
                    state = found_state.to_string();
                }
            }
            'n' => {
                let Some(current_pid) = pid else {
                    continue;
                };

                let Some((current_host, current_port)) = parse_name_field(value) else {
                    continue;
                };

                let normalized_protocol = if protocol.is_empty() {
                    "TCP".to_string()
                } else {
                    protocol.clone()
                };
                let normalized_state = if state.is_empty() {
                    "LISTEN".to_string()
                } else {
                    state.clone()
                };
                let ip_version = detect_ip_version(&current_host);
                let dedupe_key = format!(
                    "{}:{}:{}:{}",
                    current_pid, normalized_protocol, current_host, current_port
                );

                if seen.insert(dedupe_key) {
                    entries.push(RawPortEntry {
                        pid: current_pid,
                        process_name: process_name.clone(),
                        user: user.clone(),
                        protocol: normalized_protocol,
                        host: current_host,
                        port: current_port,
                        state: normalized_state,
                        ip_version,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(entries)
}

fn load_process_contexts(entries: &[RawPortEntry]) -> HashMap<i32, ProcessContext> {
    let mut pids = entries.iter().map(|entry| entry.pid).collect::<Vec<_>>();
    pids.sort();
    pids.dedup();

    pids.into_iter()
        .map(|pid| {
            let command = load_process_command(pid).unwrap_or_default();
            let started_at = load_process_started_at(pid);
            let cwd = load_process_cwd(pid);
            let ts = started_at
                .as_deref()
                .and_then(parse_ps_lstart_to_sortable_value);

            (
                pid,
                ProcessContext {
                    command,
                    cwd,
                    started_at,
                    started_at_ts: ts,
                },
            )
        })
        .collect()
}

fn load_process_command(pid: i32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args(["-ww", "-o", "command=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn load_process_started_at(pid: i32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn load_process_cwd(pid: i32) -> Option<String> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    output
        .stdout
        .split(|byte| *byte == b'\n')
        .find_map(|line| {
            let text = String::from_utf8_lossy(line);
            text.strip_prefix('n')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
}

fn summarize_hosts(hosts: &[String], ip_versions: &[String]) -> String {
    if hosts.is_empty() {
        return "-".to_string();
    }

    let version_label = if ip_versions.len() > 1 {
        "IPv4 + IPv6".to_string()
    } else {
        ip_versions.first().cloned().unwrap_or_else(|| "TCP".to_string())
    };

    if hosts.len() == 1 {
        format!("{version_label} · {}", hosts[0])
    } else {
        format!("{version_label} · {}", hosts.join("  |  "))
    }
}

fn detect_ip_version(host: &str) -> String {
    if host.contains(':') && host != "*" {
        "IPv6".to_string()
    } else {
        "IPv4".to_string()
    }
}

fn current_username() -> String {
    env::var("USER").unwrap_or_else(|_| "unknown".to_string())
}

fn process_exists(pid: i32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn normalize_kill_error(pid: i32, stderr: &str) -> String {
    if stderr.contains("Operation not permitted") {
        return format!("没有权限结束进程 {pid}，请检查进程所属用户或系统权限");
    }

    if stderr.contains("No such process") {
        return format!("进程 {pid} 已退出");
    }

    if stderr.is_empty() {
        format!("结束进程 {pid} 失败")
    } else {
        stderr.to_string()
    }
}

fn parse_name_field(value: &str) -> Option<(String, u16)> {
    let listen_target = value.rsplit("->").next().unwrap_or(value);
    let split_index = listen_target.rfind(':')?;
    let (host, port) = listen_target.split_at(split_index);
    let port = port.trim_start_matches(':').parse::<u16>().ok()?;
    Some((host.to_string(), port))
}

fn parse_ps_lstart_to_sortable_value(value: &str) -> Option<i64> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 5 {
        return None;
    }

    let month = match parts[1] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let day = parts[2].parse::<i64>().ok()?;
    let year = parts[4].parse::<i64>().ok()?;
    let time_parts = parts[3]
        .split(':')
        .map(|item| item.parse::<i64>().ok())
        .collect::<Option<Vec<_>>>()?;

    if time_parts.len() != 3 {
        return None;
    }

    Some(
        year * 10_000_000_00
            + month * 100_000_000
            + day * 1_000_000
            + time_parts[0] * 10_000
            + time_parts[1] * 100
            + time_parts[2],
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![list_ports, kill_process])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
