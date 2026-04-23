use std::process::{Command, ExitStatus, Stdio};

const LINUX_SERVICE: &str = "lirays-scada.service";
const MACOS_SERVICE: &str = "system/com.lirays.liraysscada";

pub(super) async fn restart_service() -> Result<(), String> {
    match std::env::consts::OS {
        "linux" => restart_linux_systemd(),
        "macos" => restart_macos_launchd(),
        other => Err(format!(
            "Unsupported platform: {other}. restart-service currently supports macOS and Linux."
        )),
    }
}

fn restart_linux_systemd() -> Result<(), String> {
    ensure_command_exists("systemctl")?;

    let active = run_status_quiet(
        "systemctl",
        &["is-active", "--quiet", LINUX_SERVICE],
        "checking service state",
    )?;
    if !active.success() {
        return Err(format!(
            "Service '{}' is not running (or not installed). Nothing to restart.",
            LINUX_SERVICE
        ));
    }

    let status = run_privileged_status("systemctl", &["restart", LINUX_SERVICE])?;
    if status.success() {
        println!("Service '{}' restarted successfully.", LINUX_SERVICE);
        Ok(())
    } else {
        Err(format!(
            "Failed to restart '{}'. Exit status: {}",
            LINUX_SERVICE,
            format_status(status)
        ))
    }
}

fn restart_macos_launchd() -> Result<(), String> {
    ensure_command_exists("launchctl")?;

    let print_status = run_status_quiet(
        "launchctl",
        &["print", MACOS_SERVICE],
        "checking launchd service state",
    )?;
    if !print_status.success() {
        return Err(format!(
            "Service '{}' is not loaded/running. Nothing to restart.",
            MACOS_SERVICE
        ));
    }

    let status = run_privileged_status("launchctl", &["kickstart", "-k", MACOS_SERVICE])?;
    if status.success() {
        println!("Service '{}' restarted successfully.", MACOS_SERVICE);
        Ok(())
    } else {
        Err(format!(
            "Failed to restart '{}'. Exit status: {}",
            MACOS_SERVICE,
            format_status(status)
        ))
    }
}

fn ensure_command_exists(cmd: &str) -> Result<(), String> {
    let status = Command::new("sh")
        .args(["-c", &format!("command -v {cmd} >/dev/null 2>&1")])
        .status()
        .map_err(|e| format!("Failed to check command '{cmd}': {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Required command '{cmd}' was not found in PATH."))
    }
}

fn run_status(cmd: &str, args: &[&str], context: &str) -> Result<ExitStatus, String> {
    Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("Error while {context}: {e}"))
}

fn run_status_quiet(cmd: &str, args: &[&str], context: &str) -> Result<ExitStatus, String> {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Error while {context}: {e}"))
}

fn run_privileged_status(cmd: &str, args: &[&str]) -> Result<ExitStatus, String> {
    if should_use_sudo() {
        let mut sudo_args: Vec<&str> = Vec::with_capacity(args.len() + 1);
        sudo_args.push(cmd);
        sudo_args.extend_from_slice(args);
        run_status("sudo", &sudo_args, "restarting service")
    } else {
        run_status(cmd, args, "restarting service")
    }
}

fn should_use_sudo() -> bool {
    if !command_exists("sudo") {
        return false;
    }

    let output = Command::new("id")
        .arg("-u")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(out) => {
            let uid = String::from_utf8_lossy(&out.stdout);
            uid.trim() != "0"
        }
        Err(_) => true,
    }
}

fn command_exists(cmd: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {cmd} >/dev/null 2>&1")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn format_status(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => code.to_string(),
        None => "terminated by signal".to_string(),
    }
}
