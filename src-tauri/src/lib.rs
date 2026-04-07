use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// Kill process polling configuration
const KILL_POLL_MAX_ATTEMPTS: u32 = 10;
const KILL_POLL_INTERVALS_MS: [u64; 10] = [50, 50, 50, 100, 100, 100, 200, 200, 200, 200];

// Command execution timeout configuration
const LSOF_TIMEOUT_SECS: u64 = 5;
const PS_TIMEOUT_SECS: u64 = 3;
const KILL_TIMEOUT_SECS: u64 = 2;

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReboundCheckResult {
    port: u16,
    occupied: bool,
    rebound: bool,
    same_process_name: bool,
    pid: Option<i32>,
    process_name: Option<String>,
    command: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Clone)]
struct ProcessContext {
    process_name: String,
    command: String,
    cwd: Option<String>,
    started_at: Option<String>,
    started_at_ts: Option<i64>,
}

/// Execute a command with timeout
/// Returns stdout as String on success, or error message on failure/timeout
fn run_command_with_timeout(
    cmd_path: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<String, String> {
    let (tx, rx) = mpsc::channel();
    let cmd_path_owned = cmd_path.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    // Spawn command execution in a separate thread
    thread::spawn(move || {
        let result = Command::new(&cmd_path_owned)
            .args(&args)
            .output()
            .map_err(|e| format!("failed to spawn {}: {}", cmd_path_owned, e))
            .and_then(|output| {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    Err(if stderr.is_empty() {
                        format!("{} returned non-zero status", cmd_path_owned)
                    } else {
                        stderr
                    })
                }
            });
        let _ = tx.send(result);
    });

    // Wait for result with timeout
    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(result) => result,
        Err(_) => Err(format!("{} execution timeout ({}s)", cmd_path, timeout_secs)),
    }
}

/// Helper: run command and convert Result to Option, silently ignoring errors
fn run_command_optional(cmd_path: &str, args: &[&str], timeout_secs: u64) -> Option<String> {
    run_command_with_timeout(cmd_path, args, timeout_secs).ok()
}

#[tauri::command]
fn list_ports() -> Result<PortListResponse, String> {
    let raw_entries = list_raw_ports()?;
    let contexts = load_process_contexts(&raw_entries);
    let mut grouped: HashMap<String, PortProcess> = HashMap::new();

    for entry in raw_entries {
        let key = format!("{}:{}:{}", entry.pid, entry.protocol, entry.port);
        let context = contexts.get(&entry.pid).cloned().unwrap_or(ProcessContext {
            process_name: entry.process_name.clone(),
            command: entry.process_name.clone(),
            cwd: None,
            started_at: None,
            started_at_ts: None,
        });

        let item = grouped.entry(key).or_insert_with(|| PortProcess {
            pid: entry.pid,
            process_name: context.process_name.clone(),
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

    // Verify process exists before attempting to kill
    if !process_exists(pid) {
        return Err(format!("进程 {pid} 不存在"));
    }

    let force = force.unwrap_or(false);
    let signal = if force { "-KILL" } else { "-TERM" };

    // Execute kill command with timeout
    let result = run_command_with_timeout(
        "/bin/kill",
        &[signal, &pid.to_string()],
        KILL_TIMEOUT_SECS,
    );

    // Check if kill command succeeded
    if result.is_ok() {
        // Poll with progressive delays: faster checks initially, slower later
        for attempt in 0..KILL_POLL_MAX_ATTEMPTS {
            if !process_exists(pid) {
                return Ok(KillResult {
                    pid,
                    signal: signal.trim_start_matches('-').to_string(),
                    success: true,
                    message: "已结束".to_string(),
                });
            }

            let delay_ms = KILL_POLL_INTERVALS_MS[attempt as usize];
            thread::sleep(Duration::from_millis(delay_ms));
        }

        return Err(if force {
            format!("进程 {pid} 在强制结束后仍未退出")
        } else {
            format!("进程 {pid} 未响应结束信号，请尝试强制结束")
        });
    }

    // Kill command failed, normalize the error message
    let stderr = result.unwrap_err();
    Err(normalize_kill_error(pid, &stderr))
}

#[tauri::command]
fn check_port_rebound(
    port: u16,
    previous_pid: i32,
    previous_process_name: String,
) -> Result<ReboundCheckResult, String> {
    if port == 0 {
        return Err("无效的端口".to_string());
    }

    let mut entries = list_raw_ports()?
        .into_iter()
        .filter(|entry| entry.port == port)
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return Ok(ReboundCheckResult {
            port,
            occupied: false,
            rebound: false,
            same_process_name: false,
            pid: None,
            process_name: None,
            command: None,
            message: None,
        });
    }

    entries.sort_by(|left, right| left.pid.cmp(&right.pid));

    let contexts = load_process_contexts(&entries);
    let matched_entry = entries
        .iter()
        .find(|entry| entry.pid != previous_pid && process_name_matches(&entry.process_name, &previous_process_name))
        .or_else(|| entries.iter().find(|entry| entry.pid != previous_pid))
        .or_else(|| entries.first());

    let Some(entry) = matched_entry else {
        return Ok(ReboundCheckResult {
            port,
            occupied: false,
            rebound: false,
            same_process_name: false,
            pid: None,
            process_name: None,
            command: None,
            message: None,
        });
    };

    let context = contexts.get(&entry.pid);
    let process_name = context
        .map(|item| item.process_name.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entry.process_name.clone());
    let same_process_name = process_name_matches(&process_name, &previous_process_name)
        || process_name_matches(&entry.process_name, &previous_process_name);
    let rebound = entry.pid != previous_pid;
    let command = context
        .map(|item| item.command.clone())
        .filter(|value| !value.is_empty());
    let message = if same_process_name {
        Some(format!(
            "端口 {port} 已被重新监听，当前仍是“{process_name}”（PID {}），可能由后台服务自动拉起",
            entry.pid
        ))
    } else {
        Some(format!(
            "端口 {port} 已被重新占用，当前进程为“{process_name}”（PID {}）",
            entry.pid
        ))
    };

    Ok(ReboundCheckResult {
        port,
        occupied: true,
        rebound,
        same_process_name,
        pid: Some(entry.pid),
        process_name: Some(process_name),
        command,
        message,
    })
}

fn list_raw_ports() -> Result<Vec<RawPortEntry>, String> {
    let stdout = run_command_with_timeout(
        "/usr/sbin/lsof",
        &["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcLnPTu"],
        LSOF_TIMEOUT_SECS,
    )?;

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

    if pids.is_empty() {
        return HashMap::new();
    }

    // Batch load process info using a single ps command
    let ps_data = load_process_info_batch(&pids);

    // Load cwd for each process (still needs individual lsof calls)
    pids.into_iter()
        .map(|pid| {
            let (command, started_at, started_at_ts) = ps_data
                .get(&pid)
                .cloned()
                .unwrap_or_else(|| (String::new(), None, None));

            let process_name = derive_process_name(&command).unwrap_or_default();
            let cwd = load_process_cwd(pid);

            (
                pid,
                ProcessContext {
                    process_name,
                    command,
                    cwd,
                    started_at,
                    started_at_ts,
                },
            )
        })
        .collect()
}

/// Batch load process command and start time for multiple PIDs
/// Returns HashMap<pid, (command, started_at, started_at_ts)>
fn load_process_info_batch(pids: &[i32]) -> HashMap<i32, (String, Option<String>, Option<i64>)> {
    if pids.is_empty() {
        return HashMap::new();
    }

    // Build comma-separated PID list for ps command
    let pid_list = pids
        .iter()
        .map(|pid| pid.to_string())
        .collect::<Vec<_>>()
        .join(",");

    // Execute single ps command to get all process info
    let stdout = match run_command_with_timeout(
        "/bin/ps",
        &["-ww", "-o", "pid=,lstart=,command=", "-p", &pid_list],
        PS_TIMEOUT_SECS,
    ) {
        Ok(output) => output,
        Err(_) => return HashMap::new(),
    };

    let mut result = HashMap::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse line format: "PID  Mon Jan 15 14:23:45 2024  /path/to/command args"
        // PID is right-aligned in a field, followed by lstart (5 tokens), then command
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 7 {
            continue;
        }

        // Extract PID (first token)
        let pid = match parts[0].parse::<i32>() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Extract lstart (next 5 tokens: weekday, month, day, time, year)
        let started_at = parts[1..6].join(" ");
        let started_at_ts = parse_ps_lstart_to_sortable_value(&started_at);

        // Extract command (remaining tokens)
        let command = parts[6..].join(" ");

        result.insert(
            pid,
            (
                command,
                Some(started_at),
                started_at_ts,
            ),
        );
    }

    result
}

fn derive_process_name(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((name, _)) = trimmed.split_once(" --") {
        let candidate = name.trim();
        if !candidate.is_empty() {
            return Some(candidate.to_string());
        }
    }

    trimmed
        .split_whitespace()
        .next()
        .map(|token| token.rsplit('/').next().unwrap_or(token).to_string())
        .filter(|value| !value.is_empty())
}

fn load_process_cwd(pid: i32) -> Option<String> {
    let stdout = run_command_optional(
        "/usr/sbin/lsof",
        &["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"],
        LSOF_TIMEOUT_SECS,
    )?;

    stdout
        .as_bytes()
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
    run_command_optional("/bin/kill", &["-0", &pid.to_string()], KILL_TIMEOUT_SECS).is_some()
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

fn process_name_matches(current: &str, expected: &str) -> bool {
    let current = current.trim();
    let expected = expected.trim();

    !current.is_empty() && !expected.is_empty() && current.eq_ignore_ascii_case(expected)
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
        .invoke_handler(tauri::generate_handler![
            list_ports,
            kill_process,
            check_port_rebound
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
