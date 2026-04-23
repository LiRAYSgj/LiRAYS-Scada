use std::{env, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    RemoveAdmin,
    CreateAdmin,
    RemoveOperator,
    CreateOperator,
    UpdateOperatorPassword,
    UpdateAdminPassword,
    GenerateAdminToken,
    GenerateOperatorToken,
    ListTokens,
    RevokeToken,
    Settings,
    SettingsUpdate,
    RestartService,
    WatchMetrics,
}

#[derive(Debug, Clone)]
pub(super) struct CliArgs {
    pub(super) config_path: Option<PathBuf>,
    pub(super) command: Command,
    pub(super) args: Vec<String>,
}

pub(super) fn parse_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1);
    let mut config_path: Option<PathBuf> = None;
    let mut positionals: Vec<String> = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--settings" | "-s" | "--config" | "-c" => {
                let Some(path) = args.next() else {
                    return Err("--settings <path> requires a file path".into());
                };
                config_path = Some(PathBuf::from(path));
            }
            _ if arg.starts_with("--settings=") => {
                let (_, path) = arg.split_at("--settings=".len());
                if path.is_empty() {
                    return Err("--settings=<path> requires a file path".into());
                }
                config_path = Some(PathBuf::from(path));
            }
            _ if arg.starts_with("--config=") => {
                let (_, path) = arg.split_at("--config=".len());
                if path.is_empty() {
                    return Err("--settings=<path> requires a file path".into());
                }
                config_path = Some(PathBuf::from(path));
            }
            "--help" | "-h" => return Err(usage()),
            other if other.starts_with('-') => {
                return Err(format!("Unknown flag: {other}\n{}", usage()));
            }
            other => positionals.push(other.to_string()),
        }
    }

    let (command_name, rest) = positionals.split_first().ok_or_else(usage)?;
    let command = parse_command(command_name)?;

    Ok(CliArgs {
        config_path,
        command,
        args: rest.to_vec(),
    })
}

fn parse_command(value: &str) -> Result<Command, String> {
    match value {
        "remove-admin" => Ok(Command::RemoveAdmin),
        "create-admin" => Ok(Command::CreateAdmin),
        "create-operator" => Ok(Command::CreateOperator),
        "remove-operator" => Ok(Command::RemoveOperator),
        "update-operator-password" => Ok(Command::UpdateOperatorPassword),
        "update-admin-password" => Ok(Command::UpdateAdminPassword),
        "generate-admin-token" => Ok(Command::GenerateAdminToken),
        "generate-operator-token" => Ok(Command::GenerateOperatorToken),
        "list-tokens" => Ok(Command::ListTokens),
        "revoke-token" => Ok(Command::RevokeToken),
        "settings" => Ok(Command::Settings),
        "settings-update" => Ok(Command::SettingsUpdate),
        "restart-service" | "service-restart" => Ok(Command::RestartService),
        "watch-metrics" | "metrics-watch" => Ok(Command::WatchMetrics),
        other => Err(format!("Unknown command: {other}")),
    }
}

fn usage() -> String {
    "Usage: liraysctl [--settings <path>] <command>\n\n\
     Commands:\n\
       remove-admin               Remove the 'admin' user so the web UI prompts for a new admin password on next visit.\n\
       create-admin               Create the 'admin' user if missing; fails if it already exists.\n\
       create-operator            Create the 'operator' user if missing; fails if it already exists.\n\
       remove-operator            Delete the 'operator' user.\n\
       update-operator-password   Update password for the 'operator' user.\n\
       update-admin-password      Update password for the 'admin' user.\n\
       generate-admin-token       Print a PAT token for the admin user.\n\
       generate-operator-token    Print a PAT token for the operator user.\n\
       list-tokens                List existing PAT tokens.\n\
       revoke-token               Revoke a PAT token by id or name.\n\
       settings                   Print current settings grouped by section.\n\
       settings-update            Update a setting value. Example: settings-update server.bind_host 0.0.0.0\n\
       restart-service            Restart LiRAYS service if currently running (macOS/Linux).\n\
       watch-metrics              Continuously refresh {data_dir}/metrics/metrics_rt.txt."
        .to_string()
}
