// SPDX-License-Identifier: Apache-2.0

mod chat;
mod cmd;
mod code;
mod config;
mod error;
mod git;
mod json_schema;
mod ollama;
mod patch_review;

use self::{
    chat::CommandChat, code::CommandCode, config::TheyaConfig,
    error::TheyaError, patch_review::CommandPatchReview,
};

#[tokio::main]
async fn main() -> Result<(), TheyaError> {
    let mut cli_cmd = clap::Command::new("theya")
        .about("Theya -- Offline Coding Assistant")
        .arg_required_else_help(true)
        .arg(
            clap::Arg::new("quiet")
                .short('q')
                .action(clap::ArgAction::SetTrue)
                .help("Disable logging")
                .global(true),
        )
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .action(clap::ArgAction::Count)
                .help("Increase verbose level")
                .global(true),
        )
        .subcommand_required(true)
        .subcommand(CommandPatchReview::new_cmd())
        .subcommand(CommandChat::new_cmd())
        .subcommand(CommandCode::new_cmd());

    let matches = cli_cmd.get_matches_mut();

    let (log_groups, log_level) = match matches.get_count("verbose") {
        0 => (vec!["theya", "reqwest"], log::LevelFilter::Info),
        1 => (vec!["theya", "reqwest"], log::LevelFilter::Debug),
        2 => (vec!["theya", "reqwest"], log::LevelFilter::Trace),
        _ => (vec![""], log::LevelFilter::Trace),
    };

    let config = TheyaConfig::load()?;
    log::debug!("Loaded config {config:?}");

    if !matches.get_flag("quiet") {
        let mut log_builder = env_logger::Builder::new();
        if log_groups.is_empty() {
            log_builder.filter(None, log_level);
        } else {
            for log_group in log_groups {
                log_builder.filter(Some(log_group), log_level);
            }
        }
        log_builder.init();
    }

    log::info!("theya version: {}", clap::crate_version!());

    if let Err(e) = handle_subcommand(&matches, &config).await {
        eprintln!("{e}");
        std::process::exit(1);
    }

    Ok(())
}

async fn handle_subcommand(
    matches: &clap::ArgMatches,
    config: &TheyaConfig,
) -> Result<(), TheyaError> {
    if matches
        .subcommand_matches(CommandPatchReview::CMD)
        .is_some()
    {
        CommandPatchReview::handle(&config.patch_review).await?;
        Ok(())
    } else if let Some(matches) = matches.subcommand_matches(CommandChat::CMD) {
        CommandChat::handle(matches, config).await?;
        Ok(())
    } else if matches.subcommand_matches(CommandCode::CMD).is_some() {
        CommandCode::handle(&config.code, &config.projects).await?;
        Ok(())
    } else {
        Err(TheyaError::from("Unknown command"))
    }
}
