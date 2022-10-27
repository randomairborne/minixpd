use sqlx::query;
use std::{collections::HashMap, fmt::Write};
use twilight_model::{
    application::interaction::application_command::{
        CommandData, CommandDataOption, CommandOptionValue,
    },
    channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::AppState;

pub async fn process_anvil(
    data: CommandData,
    _invoker: &User,
    state: AppState,
) -> Result<InteractionResponseData, Error> {
    let guild_id = data.guild_id.ok_or(Error::MissingGuildId)?;
    for maybe_group in data.options {
        if let CommandOptionValue::SubCommandGroup(group) = maybe_group.value {
            match maybe_group.name.as_str() {
                "rewards" => return process_rewards(group, state, guild_id).await,
                _ => return Err(Error::UnknownSubcommand),
            }
        }
    }
    Err(Error::InvalidSubcommand)
}

async fn process_rewards<'a>(
    options: Vec<CommandDataOption>,
    state: AppState,
    guild_id: Id<GuildMarker>,
) -> Result<InteractionResponseData, Error> {
    for maybe_cmd in options {
        let cmd_name = maybe_cmd.name.clone();
        if let CommandOptionValue::SubCommand(opts) = maybe_cmd.value {
            let args: HashMap<String, CommandOptionValue> =
                opts.into_iter().map(|val| (val.name, val.value)).collect();
            let contents = match cmd_name.as_str() {
                "add" => process_rewards_add(args, state, guild_id).await,
                "remove" => process_rewards_rm(args, state, guild_id).await,
                "list" => process_rewards_list(state, guild_id).await,
                _ => return Err(Error::UnknownSubcommand),
            }?;
            return Ok(InteractionResponseDataBuilder::new()
                .content(contents)
                .flags(MessageFlags::EPHEMERAL)
                .build());
        }
    }
    Err(Error::InvalidSubcommand)
}

async fn process_rewards_add(
    options: HashMap<String, CommandOptionValue>,
    state: AppState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    let level_requirement = if let CommandOptionValue::Integer(requirement) = options
        .get("requirement")
        .ok_or(Error::MissingRequiredArgument("requirement"))?
    {
        requirement
    } else {
        return Err(Error::WrongArgumentType("requirement"));
    };
    let role_id = if let CommandOptionValue::Role(role) = options
        .get("role")
        .ok_or(Error::MissingRequiredArgument("role"))?
    {
        role
    } else {
        return Err(Error::WrongArgumentType("role"));
    };
    query!(
        "INSERT INTO role_rewards (id, requirement, guild) VALUES (?, ?, ?)",
        role_id.get(),
        level_requirement,
        guild_id.get()
    )
    .execute(&state.db)
    .await?;
    Ok(format!(
        "Added role reward <@{}> at level {}!",
        role_id, level_requirement
    ))
}
async fn process_rewards_rm(
    options: HashMap<String, CommandOptionValue>,
    state: AppState,
    guild_id: Id<GuildMarker>,
) -> Result<String, Error> {
    if let Some(CommandOptionValue::Role(role)) = options.get("role") {
        query!(
            "DELETE FROM role_rewards WHERE id = ? AND guild = ?",
            role.get(),
            guild_id.get()
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward <@{}>!", role));
    } else if let Some(CommandOptionValue::Integer(level)) = options.get("level") {
        query!(
            "DELETE FROM role_rewards WHERE requirement = ? AND guild = ?",
            level,
            guild_id.get()
        )
        .execute(&state.db)
        .await?;
        return Ok(format!("Removed role reward for level {level}!"));
    };
    Err(Error::WrongArgumentCount(
        "`/anvil rewards remove` requires either a level or a role!",
    ))
}
async fn process_rewards_list(state: AppState, guild_id: Id<GuildMarker>) -> Result<String, Error> {
    let roles = query!("SELECT * FROM role_rewards WHERE guild = ?", guild_id.get())
        .fetch_all(&state.db)
        .await?;
    let mut data = String::new();
    for role in roles {
        writeln!(
            data,
            "Role reward <@&{}> at level {}",
            role.id, role.requirement
        )?;
    }
    Ok(data)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Discord sent an invalid subcommand!")]
    InvalidSubcommand,
    #[error("Discord sent an unknown subcommand!")]
    UnknownSubcommand,
    #[error("Discord did not send required argument {0}!")]
    MissingRequiredArgument(&'static str),
    #[error("Discord sent wrong type for required argument {0}!")]
    WrongArgumentType(&'static str),
    #[error("Discord did not send a guild ID!")]
    MissingGuildId,
    #[error("Command had wrong number of arguments: {0}!")]
    WrongArgumentCount(&'static str),
    #[error("SQLx encountered an error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Rust writeln! returned an error: {0}")]
    Fmt(#[from] std::fmt::Error),
}
