use crate::AppState;
use twilight_interactions::command::CommandModel;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{
            application_command::CommandData, Interaction, InteractionData, InteractionType,
        },
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{marker::GuildMarker, Id},
    user::User,
};

pub async fn process(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    Ok(if interaction.kind == InteractionType::ApplicationCommand {
        process_app_cmd(interaction, state).await?
    } else {
        InteractionResponse {
            kind: InteractionResponseType::Pong,
            data: None,
        }
    })
}

async fn process_app_cmd(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    #[cfg(debug_assertions)]
    trace!("{interaction:#?}");
    let data = if let Some(data) = interaction.data {
        if let InteractionData::ApplicationCommand(cmd) = data {
            *cmd
        } else {
            return Err(CommandProcessorError::WrongInteractionData);
        }
    } else {
        return Err(CommandProcessorError::NoInteractionData);
    };
    let invoker = match interaction.member {
        Some(val) => val.user,
        None => interaction.user,
    }
    .ok_or(CommandProcessorError::NoInvoker)?;
    match data.kind {
        CommandType::ChatInput => {
            process_slash_cmd(
                data,
                interaction.token,
                interaction.guild_id,
                invoker,
                state,
            )
            .await
        }
        CommandType::User => process_user_cmd(data, interaction.token, invoker, state).await,
        CommandType::Message => process_msg_cmd(data, interaction.token, invoker, state).await,
        _ => Err(CommandProcessorError::WrongInteractionData),
    }
}

async fn process_slash_cmd(
    data: CommandData,
    token: String,
    guild_id: Option<Id<GuildMarker>>,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    let guild_id = guild_id.ok_or(CommandProcessorError::NoGuildId)?;
    match data.name.as_str() {
        "rank" => {
            let target = crate::cmd_defs::RankCommand::from_interaction(data.into())?
                .user
                .map_or_else(|| invoker.clone(), |v| v.resolved);
            crate::levels::get_level(guild_id, target, invoker, token, state).await
        }
        "leaderboard" => crate::levels::leaderboard(guild_id, state).await,
        "toy" => {
            let selected = crate::cmd_defs::ToyCommand::from_interaction(data.into())?.toy_image;
            crate::toy::modify(selected, guild_id, invoker, state).await
        }
        _ => Err(CommandProcessorError::UnrecognizedCommand),
    }
}

async fn process_user_cmd(
    data: CommandData,
    token: String,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    let msg_id = data
        .target_id
        .ok_or(CommandProcessorError::NoMessageTargetId)?;
    let user = data
        .resolved
        .as_ref()
        .ok_or(CommandProcessorError::NoResolvedData)?
        .users
        .get(&msg_id.cast())
        .ok_or(CommandProcessorError::NoTarget)?;
    crate::levels::get_level(
        data.guild_id.ok_or(CommandProcessorError::NoGuildId)?,
        user.clone(),
        invoker,
        token,
        state,
    )
    .await
}

async fn process_msg_cmd(
    data: CommandData,
    token: String,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, CommandProcessorError> {
    let msg_id = data
        .target_id
        .ok_or(CommandProcessorError::NoMessageTargetId)?;
    let user = &data
        .resolved
        .as_ref()
        .ok_or(CommandProcessorError::NoResolvedData)?
        .messages
        .get(&msg_id.cast())
        .ok_or(CommandProcessorError::NoTarget)?
        .author;
    crate::levels::get_level(
        data.guild_id.ok_or(CommandProcessorError::NoGuildId)?,
        user.clone(),
        invoker,
        token,
        state,
    )
    .await
}

#[derive(Debug, thiserror::Error)]
pub enum CommandProcessorError {
    #[error("Discord sent a command that is not known!")]
    UnrecognizedCommand,
    #[error("Discord did not send a user object for the command invoker when it was required!")]
    NoInvoker,
    #[error("Discord did not send a user object for the command target when it was required!")]
    NoTarget,
    #[error("Discord did not send part of the Resolved Data!")]
    NoResolvedData,
    #[error("Discord did not send target ID for message!")]
    NoMessageTargetId,
    #[error("Discord sent interaction data for an unsupported interaction type!")]
    WrongInteractionData,
    #[error("Discord did not send any interaction data!")]
    NoInteractionData,
    #[error("Discord did not send a guild ID!")]
    NoGuildId,
    #[error("Interaction parser encountered an error: {0}!")]
    Parse(#[from] twilight_interactions::error::ParseError),
    #[error("SVG renderer encountered an error: {0}!")]
    ImageGenerator(#[from] crate::render_card::RenderingError),
    #[error("SQLx encountered an error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
