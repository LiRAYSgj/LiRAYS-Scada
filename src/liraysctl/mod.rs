mod args;
mod config;
mod db;
mod metrics;
mod prompt;
mod service;
mod settings;
mod tokens;
mod users;

use args::{CliArgs, Command};

pub async fn run() -> Result<(), String> {
    let CliArgs {
        config_path,
        command,
        args,
    } = args::parse_args()?;

    match command {
        Command::RemoveAdmin => {
            expect_no_extra_args(&args, "remove-admin")?;
            users::remove_admin(config_path).await
        }
        Command::CreateAdmin => {
            expect_no_extra_args(&args, "create-admin")?;
            users::create_admin(config_path).await
        }
        Command::RemoveOperator => {
            expect_no_extra_args(&args, "remove-operator")?;
            users::remove_operator(config_path).await
        }
        Command::CreateOperator => {
            expect_no_extra_args(&args, "create-operator")?;
            users::create_operator(config_path).await
        }
        Command::UpdateOperatorPassword => {
            expect_no_extra_args(&args, "update-operator-password")?;
            users::update_operator_password(config_path).await
        }
        Command::UpdateAdminPassword => {
            expect_no_extra_args(&args, "update-admin-password")?;
            users::update_admin_password(config_path).await
        }
        Command::GenerateAdminToken => {
            expect_no_extra_args(&args, "generate-admin-token")?;
            tokens::generate_admin_token(config_path).await
        }
        Command::GenerateOperatorToken => {
            expect_no_extra_args(&args, "generate-operator-token")?;
            tokens::generate_operator_token(config_path).await
        }
        Command::ListTokens => {
            expect_no_extra_args(&args, "list-tokens")?;
            tokens::list_tokens(config_path).await
        }
        Command::RevokeToken => {
            expect_no_extra_args(&args, "revoke-token")?;
            tokens::revoke_token(config_path).await
        }
        Command::Settings => {
            expect_no_extra_args(&args, "settings")?;
            settings::print_settings(config_path).await
        }
        Command::SettingsUpdate => settings::update_setting(config_path, &args).await,
        Command::RestartService => {
            expect_no_extra_args(&args, "restart-service")?;
            service::restart_service().await
        }
        Command::WatchMetrics => {
            expect_no_extra_args(&args, "watch-metrics")?;
            metrics::watch_metrics(config_path).await
        }
    }
}

fn expect_no_extra_args(args: &[String], command: &str) -> Result<(), String> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Command '{command}' does not accept extra arguments: {}",
            args.join(" ")
        ))
    }
}
