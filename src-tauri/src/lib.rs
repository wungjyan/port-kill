use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// Kill process polling configuration
const KILL_POLL_MAX_ATTEMPTS: u32 = 10;
const KILL_POLL_INTERVALS_MS: [u64; 10] = [50, 50, 50, 100, 100, 100, 200, 200, 200, 200];

// Command execution timeout configuration
const LSOF_TIMEOUT_SECS: u64 = 5;
const PS_TIMEOUT_SECS: u64 = 3;
const KILL_TIMEOUT_SECS: u64 = 2;
const DOCKER_TIMEOUT_SECS: u64 = 3;
const SELF_CHAIN_DEPTH: usize = 8;
const DOCKER_FORMAT: &str = "{{.Names}}\t{{.Image}}\t{{.Ports}}\t{{.Label \"com.docker.compose.project\"}}\t{{.Label \"com.docker.compose.service\"}}";
const DOCKER_PROXY_NAMES: [&str; 4] = [
    "com.docker.backend",
    "docker-proxy",
    "vpnkit",
    "com.docker.vpnkit",
];

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
struct PortContainer {
    name: String,
    image: String,
    project: String,
    service: String,
    container_port: Option<u16>,
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
    container: Option<PortContainer>,
    docker_proxy: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortListResponse {
    current_user: String,
    items: Vec<PortProcess>,
    docker_available: bool,
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

#[derive(Debug, Default)]
struct DockerProbe {
    available: bool,
    ports: HashMap<u16, PortContainer>,
}

/// Execute a command with timeout
/// Returns stdout as String on success, or error message on failure/timeout
fn run_command_with_timeout(
    cmd_path: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<String, String> {
    run_command_with_timeout_allowing_empty_exit(cmd_path, args, timeout_secs, None)
}

/// `lsof` uses exit code 1 to report that no files matched the query. Treat that
/// specific empty response as a successful query while preserving real errors.
fn run_lsof_with_timeout(args: &[&str], timeout_secs: u64) -> Result<String, String> {
    run_command_with_timeout_allowing_empty_exit("/usr/sbin/lsof", args, timeout_secs, Some(1))
}

fn run_command_with_timeout_allowing_empty_exit(
    cmd_path: &str,
    args: &[&str],
    timeout_secs: u64,
    empty_result_exit_code: Option<i32>,
) -> Result<String, String> {
    let mut child = Command::new(cmd_path)
        .args(args)
        // `ps -o lstart=` follows the caller's locale. Without a fixed C locale,
        // localized month names make the start-time value unparseable and turn
        // the PID-reuse check into an invisible skip.
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法执行 {cmd_path}：{error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("无法读取 {cmd_path} 的标准输出"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("无法读取 {cmd_path} 的错误输出"))?;
    let stdout_reader = thread::spawn(move || read_command_stream(stdout));
    let stderr_reader = thread::spawn(move || read_command_stream(stderr));

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(format!("{cmd_path} 执行超时（{timeout_secs}s）"));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(format!("等待 {cmd_path} 执行结果失败：{error}"));
            }
        }
    };

    let stdout = stdout_reader
        .join()
        .map_err(|_| format!("读取 {cmd_path} 的标准输出失败"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| format!("读取 {cmd_path} 的错误输出失败"))??;

    let stdout_is_empty = stdout.iter().all(u8::is_ascii_whitespace);
    let stderr_is_empty = stderr.iter().all(u8::is_ascii_whitespace);
    let is_accepted_empty_result =
        empty_result_exit_code == status.code() && stdout_is_empty && stderr_is_empty;

    if status.success() || is_accepted_empty_result {
        Ok(String::from_utf8_lossy(&stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("{cmd_path} 执行失败，退出状态为 {status}")
        } else {
            stderr
        })
    }
}

fn read_command_stream(mut stream: impl Read) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    stream
        .read_to_end(&mut buffer)
        .map_err(|error| format!("读取命令输出失败：{error}"))?;
    Ok(buffer)
}

/// Helper: run command and convert Result to Option, silently ignoring errors
fn run_command_optional(cmd_path: &str, args: &[&str], timeout_secs: u64) -> Option<String> {
    run_command_with_timeout(cmd_path, args, timeout_secs).ok()
}

#[tauri::command]
async fn list_ports() -> Result<PortListResponse, String> {
    tauri::async_runtime::spawn_blocking(list_ports_blocking)
        .await
        .map_err(|error| format!("端口扫描任务异常结束：{error}"))?
}

fn list_ports_blocking() -> Result<PortListResponse, String> {
    let (raw_entries, docker) = thread::scope(|scope| {
        let docker_handle = scope.spawn(probe_docker);
        let raw_result = list_raw_ports();
        let docker = docker_handle.join().unwrap_or_default();
        raw_result.map(|entries| (entries, docker))
    })?;
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
            container: None,
            docker_proxy: false,
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
            item.container = docker.ports.get(&item.port).cloned();
            item.docker_proxy =
                item.container.is_some() || is_docker_proxy_process(&item.process_name);
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
        docker_available: docker.available,
    })
}

#[tauri::command]
async fn kill_process(
    pid: i32,
    force: Option<bool>,
    expected_port: u16,
    expected_started_at_ts: Option<i64>,
    expected_process_name: String,
    expected_user: String,
) -> Result<KillResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        kill_process_blocking(
            pid,
            force,
            expected_port,
            expected_started_at_ts,
            &expected_process_name,
            &expected_user,
        )
    })
    .await
    .map_err(|error| format!("结束进程任务异常结束：{error}"))?
}

fn kill_process_blocking(
    pid: i32,
    force: Option<bool>,
    expected_port: u16,
    expected_started_at_ts: Option<i64>,
    expected_process_name: &str,
    expected_user: &str,
) -> Result<KillResult, String> {
    if pid <= 0 {
        return Err("无效的 PID".to_string());
    }

    if expected_port == 0 {
        return Err("无效的端口".to_string());
    }

    validate_process_identity(
        pid,
        expected_port,
        expected_started_at_ts,
        expected_process_name,
        expected_user,
    )?;

    validate_not_docker_proxy(expected_port, expected_process_name)?;

    // Verify process exists before attempting to kill
    if !process_exists(pid) {
        return Err(format!("进程 {pid} 不存在"));
    }

    let force = force.unwrap_or(false);
    let signal = if force { "-KILL" } else { "-TERM" };

    // Execute kill command with timeout
    let result =
        run_command_with_timeout("/bin/kill", &[signal, &pid.to_string()], KILL_TIMEOUT_SECS);

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
async fn check_port_rebound(
    port: u16,
    previous_pid: i32,
    previous_process_name: String,
) -> Result<ReboundCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        check_port_rebound_blocking(port, previous_pid, &previous_process_name)
    })
    .await
    .map_err(|error| format!("端口回弹检查任务异常结束：{error}"))?
}

fn check_port_rebound_blocking(
    port: u16,
    previous_pid: i32,
    previous_process_name: &str,
) -> Result<ReboundCheckResult, String> {
    if port == 0 {
        return Err("无效的端口".to_string());
    }

    let mut entries = list_raw_ports_for_port(Some(port))?;

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

    entries.sort_by_key(|entry| entry.pid);

    let contexts = load_process_contexts(&entries);
    let matched_entry = entries
        .iter()
        .find(|entry| {
            entry.pid != previous_pid
                && process_name_matches(&entry.process_name, previous_process_name)
        })
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
    let same_process_name = process_name_matches(&process_name, previous_process_name)
        || process_name_matches(&entry.process_name, previous_process_name);
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
    list_raw_ports_for_port(None)
}

fn list_raw_ports_for_port(port: Option<u16>) -> Result<Vec<RawPortEntry>, String> {
    let tcp_filter = port
        .map(|value| format!("-iTCP:{value}"))
        .unwrap_or_else(|| "-iTCP".to_string());
    let stdout = run_lsof_with_timeout(
        &["-nP", "-a", &tcp_filter, "-sTCP:LISTEN", "-F", "pcLnPTu"],
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
    let cwd_data = load_process_cwds_batch(&pids);
    let fallback_names = entries.iter().fold(HashMap::new(), |mut names, entry| {
        if !entry.process_name.is_empty() {
            names
                .entry(entry.pid)
                .or_insert_with(|| entry.process_name.clone());
        }
        names
    });

    pids.into_iter()
        .map(|pid| {
            let (command, started_at, started_at_ts) = ps_data
                .get(&pid)
                .cloned()
                .unwrap_or_else(|| (String::new(), None, None));

            let process_name = fallback_names
                .get(&pid)
                .cloned()
                .or_else(|| derive_process_name(&command))
                .unwrap_or_else(|| "未知进程".to_string());
            let command = if command.is_empty() {
                process_name.clone()
            } else {
                command
            };
            let cwd = cwd_data.get(&pid).cloned();

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

        result.insert(pid, (command, Some(started_at), started_at_ts));
    }

    result
}

fn first_command_token(command: &str) -> &str {
    if let Some(quote) = command
        .chars()
        .next()
        .filter(|value| *value == '"' || *value == '\'')
    {
        let rest = &command[quote.len_utf8()..];
        return rest.find(quote).map_or(rest, |index| &rest[..index]);
    }

    command.split_whitespace().next().unwrap_or("")
}

fn derive_process_name(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains(".app/Contents/MacOS/") {
        let executable_path = trimmed
            .split_once(" --")
            .map(|(path, _)| path)
            .unwrap_or(trimmed);
        return executable_path
            .rsplit('/')
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty() && !value.starts_with('-'))
            .map(ToString::to_string);
    }

    let token = first_command_token(trimmed);
    let executable = token.rsplit(['/', '\\']).next().unwrap_or(token).trim();

    (!executable.is_empty() && !executable.starts_with('-')).then(|| executable.to_string())
}

fn load_process_cwds_batch(pids: &[i32]) -> HashMap<i32, String> {
    if pids.is_empty() {
        return HashMap::new();
    }

    let pid_list = pids
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let Some(stdout) = run_lsof_with_timeout(
        &["-a", "-p", &pid_list, "-d", "cwd", "-Fpn"],
        LSOF_TIMEOUT_SECS,
    )
    .ok() else {
        return HashMap::new();
    };

    let mut current_pid = None;
    let mut result = HashMap::new();

    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix('p') {
            current_pid = value.trim().parse::<i32>().ok();
        } else if let (Some(pid), Some(value)) = (current_pid, line.strip_prefix('n')) {
            let value = value.trim();
            if !value.is_empty() {
                result.insert(pid, value.to_string());
            }
        }
    }

    result
}

fn summarize_hosts(hosts: &[String], ip_versions: &[String]) -> String {
    if hosts.is_empty() {
        return "-".to_string();
    }

    let version_label = if ip_versions.len() > 1 {
        "IPv4 + IPv6".to_string()
    } else {
        ip_versions
            .first()
            .cloned()
            .unwrap_or_else(|| "TCP".to_string())
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
    run_command_optional("/usr/bin/id", &["-un"], KILL_TIMEOUT_SECS)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| env::var("USER").ok())
        .unwrap_or_else(|| "unknown".to_string())
}

fn resolve_docker_binary() -> String {
    const CANDIDATES: [&str; 4] = [
        "/usr/local/bin/docker",
        "/opt/homebrew/bin/docker",
        "/Applications/Docker.app/Contents/Resources/bin/docker",
        "/usr/bin/docker",
    ];

    CANDIDATES
        .iter()
        .find(|path| Path::new(path).is_file())
        .copied()
        .unwrap_or("docker")
        .to_string()
}

fn expand_docker_port_range(value: &str) -> Option<Vec<u16>> {
    let value = value.trim();
    if let Some((start, end)) = value.split_once('-') {
        let start = start.parse::<u16>().ok()?;
        let end = end.parse::<u16>().ok()?;
        if start == 0 || end < start {
            return None;
        }
        return Some((start..=end).collect());
    }

    let port = value.parse::<u16>().ok()?;
    (port > 0).then_some(vec![port])
}

fn parse_docker_port_mappings(field: &str) -> Vec<(u16, Option<u16>)> {
    let mut mappings = Vec::new();

    for chunk in field.split(',') {
        let mapping = chunk.trim();
        let Some((left, right)) = mapping.split_once("->") else {
            continue;
        };
        let (container_text, protocol) = right.split_once('/').unwrap_or((right, "tcp"));
        if !protocol.eq_ignore_ascii_case("tcp") {
            continue;
        }
        let Some(colon_index) = left.rfind(':') else {
            continue;
        };

        let Some(host_ports) = expand_docker_port_range(&left[colon_index + 1..]) else {
            continue;
        };
        let container_ports = expand_docker_port_range(container_text);

        for (index, host_port) in host_ports.iter().copied().enumerate() {
            let container_port = container_ports.as_ref().and_then(|ports| {
                if ports.len() == host_ports.len() {
                    ports.get(index).copied()
                } else {
                    ports.first().copied()
                }
            });
            mappings.push((host_port, container_port));
        }
    }

    mappings
}

fn build_docker_port_map(stdout: &str) -> HashMap<u16, PortContainer> {
    let mut ports = HashMap::new();

    for raw_line in stdout.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() < 3 || parts[0].trim().is_empty() {
            continue;
        }

        let container = PortContainer {
            name: parts[0].trim().to_string(),
            image: parts.get(1).map_or("", |value| value.trim()).to_string(),
            project: parts.get(3).map_or("", |value| value.trim()).to_string(),
            service: parts.get(4).map_or("", |value| value.trim()).to_string(),
            container_port: None,
        };

        for (host_port, container_port) in parse_docker_port_mappings(parts[2]) {
            let mut attributed = container.clone();
            attributed.container_port = container_port;
            ports.entry(host_port).or_insert(attributed);
        }
    }

    ports
}

fn is_docker_proxy_process(process_name: &str) -> bool {
    let process_name = process_name.trim().to_ascii_lowercase();
    !process_name.is_empty()
        && DOCKER_PROXY_NAMES
            .iter()
            .any(|name| process_name.contains(name))
}

fn probe_docker() -> DockerProbe {
    let docker_binary = resolve_docker_binary();
    match run_command_with_timeout(
        &docker_binary,
        &["ps", "--no-trunc", "--format", DOCKER_FORMAT],
        DOCKER_TIMEOUT_SECS,
    ) {
        Ok(stdout) => DockerProbe {
            available: true,
            ports: build_docker_port_map(&stdout),
        },
        Err(_) => DockerProbe::default(),
    }
}

fn validate_not_docker_proxy(port: u16, process_name: &str) -> Result<(), String> {
    let docker = probe_docker();
    let container = docker.ports.get(&port);
    if container.is_none() && !is_docker_proxy_process(process_name) {
        return Ok(());
    }

    if let Some(container) = container {
        return Err(format!(
            "端口 {port} 由 Docker 端口转发进程持有，不能在这里结束。请在 Docker Desktop 中停止容器“{}”。结束转发进程不会停止容器，还会影响该容器发布的其它端口。",
            container.name
        ));
    }

    if docker.available {
        Err(format!(
            "端口 {port} 由 Docker 端口转发进程“{process_name}”持有，但没有找到对应容器。为避免中断 Docker 转发，已取消结束操作。"
        ))
    } else {
        Err(format!(
            "端口 {port} 由 Docker 端口转发进程“{process_name}”持有，但 Docker 当前不可用，无法确认背后的容器。为避免中断转发，已取消结束操作。"
        ))
    }
}

fn parse_process_parents(stdout: &str) -> HashMap<i32, i32> {
    stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse::<i32>().ok()?;
            let parent_pid = parts.next()?.parse::<i32>().ok()?;
            Some((pid, parent_pid))
        })
        .collect()
}

fn collect_process_chain(
    current_pid: i32,
    parents: &HashMap<i32, i32>,
    depth: usize,
) -> Option<HashSet<i32>> {
    let mut chain = HashSet::from([current_pid]);
    let mut cursor = current_pid;

    for _ in 0..depth {
        let parent_pid = *parents.get(&cursor)?;
        if parent_pid <= 1 {
            break;
        }
        if !chain.insert(parent_pid) {
            return None;
        }
        cursor = parent_pid;
    }

    Some(chain)
}

fn current_process_chain() -> Result<HashSet<i32>, String> {
    let current_pid = i32::try_from(std::process::id())
        .map_err(|_| "Port Kill 的 PID 超出支持范围，无法执行安全检查".to_string())?;
    let stdout = run_command_with_timeout("/bin/ps", &["-A", "-o", "pid=,ppid="], PS_TIMEOUT_SECS)
        .map_err(|error| format!("无法读取 Port Kill 的进程父子关系：{error}"))?;
    let parents = parse_process_parents(&stdout);

    collect_process_chain(current_pid, &parents, SELF_CHAIN_DEPTH).ok_or_else(|| {
        format!("无法确认 Port Kill（PID {current_pid}）所在的进程链，已取消本次操作")
    })
}

fn validate_started_at_identity(
    pid: i32,
    expected_started_at_ts: Option<i64>,
    current_started_at_ts: Option<i64>,
) -> Result<(), String> {
    let Some(expected_started_at_ts) = expected_started_at_ts else {
        return Err(format!(
            "列表中的进程 {pid} 缺少启动时间，无法确认身份，请刷新列表后重试"
        ));
    };

    let Some(current_started_at_ts) = current_started_at_ts else {
        return Err(format!(
            "无法确认进程 {pid} 的启动时间，为避免误杀已取消操作，请刷新列表后重试"
        ));
    };

    if current_started_at_ts != expected_started_at_ts {
        return Err(format!(
            "进程身份已变化：PID {pid} 的启动时间与列表记录不一致，请刷新后重试"
        ));
    }

    Ok(())
}

fn validate_process_identity(
    pid: i32,
    expected_port: u16,
    expected_started_at_ts: Option<i64>,
    expected_process_name: &str,
    expected_user: &str,
) -> Result<(), String> {
    if pid <= 1 {
        return Err("不能结束系统进程".to_string());
    }

    let current_pid = i32::try_from(std::process::id())
        .map_err(|_| "Port Kill 的 PID 超出支持范围，无法执行安全检查".to_string())?;
    if pid == current_pid {
        return Err("不能结束 Port Kill 自身的进程".to_string());
    }

    let process_chain = current_process_chain()?;
    if process_chain.contains(&pid) {
        return Err(
            "不能结束 Port Kill 所在的进程链上的进程（结束了当前界面也会一起退出）".to_string(),
        );
    }

    let current_user = current_username();
    if expected_user != current_user {
        return Err(format!(
            "没有权限结束用户“{expected_user}”的进程，请仅操作当前用户“{current_user}”的进程"
        ));
    }

    let entries = list_raw_ports_for_port(Some(expected_port))?;
    let Some(entry) = entries.iter().find(|entry| entry.pid == pid) else {
        return Err(format!(
            "进程身份已变化：PID {pid} 已不再监听端口 {expected_port}，请刷新列表后重试"
        ));
    };

    if entry.user != expected_user
        || !process_name_matches(&entry.process_name, expected_process_name)
    {
        return Err(format!(
            "进程身份已变化：端口 {expected_port} 的监听进程与列表记录不一致，请刷新后重试"
        ));
    }

    let process_info = load_process_info_batch(&[pid]);
    let current_started_at_ts = process_info
        .get(&pid)
        .and_then(|(_, _, started_at_ts)| *started_at_ts);
    validate_started_at_identity(pid, expected_started_at_ts, current_started_at_ts)?;

    Ok(())
}

fn process_exists(pid: i32) -> bool {
    match run_command_with_timeout("/bin/kill", &["-0", &pid.to_string()], KILL_TIMEOUT_SECS) {
        Ok(_) => true,
        Err(error) => !error.contains("No such process"),
    }
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
        year * 10_000_000_000
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
        .invoke_handler(tauri::generate_handler![
            list_ports,
            kill_process,
            check_port_rebound
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn returns_an_empty_list_for_an_unused_port() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("allocate an unused port");
        let port = listener.local_addr().expect("read listener address").port();
        drop(listener);

        let entries = list_raw_ports_for_port(Some(port))
            .expect("an unused port should be a successful empty lsof result");
        assert!(entries.is_empty());

        let rebound = check_port_rebound_blocking(port, i32::MAX, "unused-process")
            .expect("an unused port should produce a successful rebound check");
        assert!(!rebound.occupied);
        assert!(!rebound.rebound);
    }

    #[test]
    fn parses_ipv4_ipv6_and_wildcard_listener_names() {
        assert_eq!(
            parse_name_field("127.0.0.1:5173"),
            Some(("127.0.0.1".to_string(), 5173))
        );
        assert_eq!(
            parse_name_field("[::1]:1420"),
            Some(("[::1]".to_string(), 1420))
        );
        assert_eq!(parse_name_field("*:8080"), Some(("*".to_string(), 8080)));
    }

    #[test]
    fn derives_only_the_executable_name_from_long_commands() {
        assert_eq!(
            derive_process_name(
                "/Applications/Visual Studio Code.app/Contents/MacOS/Electron --type=utility"
            ),
            Some("Electron".to_string())
        );
        assert_eq!(
            derive_process_name("/usr/local/bin/node /project/server.js --port 3000"),
            Some("node".to_string())
        );
        assert_eq!(
            derive_process_name("\"C:\\Program Files\\nodejs\\node.exe\" --inspect"),
            Some("node.exe".to_string())
        );
        assert_eq!(derive_process_name("--mode 1 --vm-id test"), None);
    }

    #[test]
    fn parses_ps_start_time_for_sorting() {
        assert_eq!(
            parse_ps_lstart_to_sortable_value("Mon Jul 13 14:23:45 2026"),
            Some(20_260_713_142_345)
        );
        assert_eq!(parse_ps_lstart_to_sortable_value("invalid"), None);
    }

    #[test]
    fn detects_ip_versions_from_lsof_hosts() {
        assert_eq!(detect_ip_version("127.0.0.1"), "IPv4");
        assert_eq!(detect_ip_version("*"), "IPv4");
        assert_eq!(detect_ip_version("[::1]"), "IPv6");
    }

    #[test]
    fn process_name_matching_is_case_insensitive_but_not_empty() {
        assert!(process_name_matches("Node", "node"));
        assert!(!process_name_matches("", "node"));
        assert!(!process_name_matches("node", ""));
    }

    #[test]
    fn ps_start_time_is_parsed_under_a_fixed_c_locale() {
        let pid = std::process::id().to_string();
        let stdout =
            run_command_with_timeout("/bin/ps", &["-o", "lstart=", "-p", &pid], PS_TIMEOUT_SECS)
                .expect("read this process start time");

        assert!(
            parse_ps_lstart_to_sortable_value(stdout.trim()).is_some(),
            "unexpected localized ps output: {}",
            stdout.trim()
        );
    }

    #[test]
    fn rejects_missing_or_mismatched_start_times() {
        let missing_from_list = validate_started_at_identity(42, None, Some(1))
            .expect_err("a missing expected start time must be rejected");
        assert!(missing_from_list.contains("缺少启动时间"));

        let missing_live = validate_started_at_identity(42, Some(1), None)
            .expect_err("an unavailable live start time must be rejected");
        assert!(missing_live.contains("无法确认进程 42 的启动时间"));

        let changed = validate_started_at_identity(42, Some(1), Some(2))
            .expect_err("a changed start time must be rejected");
        assert!(changed.contains("启动时间与列表记录不一致"));

        assert!(validate_started_at_identity(42, Some(1), Some(1)).is_ok());
    }

    #[test]
    fn builds_the_current_process_chain_and_stops_before_pid_one() {
        let parents = HashMap::from([(500, 400), (400, 300), (300, 1)]);
        let chain = collect_process_chain(500, &parents, SELF_CHAIN_DEPTH)
            .expect("the parent chain should be readable");

        assert_eq!(chain, HashSet::from([500, 400, 300]));
        assert!(!chain.contains(&1));
        assert!(collect_process_chain(500, &HashMap::new(), SELF_CHAIN_DEPTH).is_none());
    }

    #[test]
    fn rejects_the_current_process_system_process_and_its_ancestors() {
        let current_pid = i32::try_from(std::process::id()).expect("current pid should fit in i32");
        let self_error = validate_process_identity(current_pid, 5173, Some(1), "port-kill", "user")
            .expect_err("the app must not terminate itself");
        assert!(self_error.contains("Port Kill 自身"));

        let system_error = validate_process_identity(1, 5173, Some(1), "launchd", "root")
            .expect_err("pid 1 must never be terminated");
        assert!(system_error.contains("系统进程"));

        let chain = current_process_chain().expect("read the current process chain");
        let ancestor = chain
            .iter()
            .copied()
            .find(|pid| *pid != current_pid && *pid > 1);
        if let Some(ancestor) = ancestor {
            let ancestor_error =
                validate_process_identity(ancestor, 5173, Some(1), "parent", "user")
                    .expect_err("ancestors must never be terminated");
            assert!(ancestor_error.contains("进程链"));
        }
    }

    #[test]
    fn maps_docker_published_tcp_ports_to_containers() {
        let stdout = concat!(
            "local_postgres\tpostgres:latest\t0.0.0.0:5432->5432/tcp, [::]:5432->5432/tcp\t\t\n",
            "smartagri-redis\tredis:7-alpine\t0.0.0.0:6379->6379/tcp, 127.0.0.1:5000-5001->6000-6001/tcp\tsmartagri-ai\tredis\n",
            "udp-only\timage\t0.0.0.0:5353->5353/udp\t\t\n"
        );
        let ports = build_docker_port_map(stdout);

        let postgres = ports.get(&5432).expect("postgres mapping");
        assert_eq!(postgres.name, "local_postgres");
        assert_eq!(postgres.image, "postgres:latest");
        assert_eq!(postgres.container_port, Some(5432));

        let redis = ports.get(&6379).expect("redis mapping");
        assert_eq!(redis.project, "smartagri-ai");
        assert_eq!(redis.service, "redis");
        assert_eq!(
            ports.get(&5000).and_then(|item| item.container_port),
            Some(6000)
        );
        assert_eq!(
            ports.get(&5001).and_then(|item| item.container_port),
            Some(6001)
        );
        assert!(
            !ports.contains_key(&5353),
            "UDP bindings are not TCP listeners"
        );
    }

    #[test]
    fn detects_docker_proxy_process_names() {
        assert!(is_docker_proxy_process("com.docker.backend"));
        assert!(is_docker_proxy_process("com.docker.backend.exe"));
        assert!(is_docker_proxy_process("docker-proxy"));
        assert!(!is_docker_proxy_process("node"));
        assert!(!is_docker_proxy_process(""));
    }
}
