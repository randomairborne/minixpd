use crate::{AppState, Error};
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
) -> Result<InteractionResponse, Error> {
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
) -> Result<InteractionResponse, Error> {
    #[cfg(debug_assertions)]
    trace!("{interaction:#?}");
    let data = if let Some(data) = interaction.data {
        if let InteractionData::ApplicationCommand(cmd) = data {
            *cmd
        } else {
            return Err(Error::WrongInteractionData);
        }
    } else {
        return Err(Error::NoInteractionData);
    };
    let invoker = match interaction.member {
        Some(val) => val.user,
        None => interaction.user,
    }
    .ok_or(Error::NoInvoker)?;
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
        _ => Err(Error::WrongInteractionData),
    }
}

async fn process_slash_cmd(
    data: CommandData,
    token: String,
    guild_id: Option<Id<GuildMarker>>,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    let guild_id = guild_id.ok_or(Error::NoGuildId)?;
    match data.name.as_str() {
        "rank" => {
            let target = crate::cmd_defs::RankCommand::from_interaction(data.into())?
                .user
                .map_or_else(|| invoker.clone(), |v| v.resolved);
            crate::levels::get_level(guild_id, target, invoker, token, state).await
        }
        "leaderboard" => crate::levels::leaderboard(guild_id, state.db).await,
        "toy" => {
            let selected = crate::cmd_defs::ToyCommand::from_interaction(data.into())?.toy_image;
            crate::toy::modify(selected, guild_id, invoker, state).await
        }
        _ => Err(Error::UnrecognizedCommand),
    }
}

async fn process_user_cmd(
    data: CommandData,
    token: String,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    let msg_id = data.target_id.ok_or(Error::NoMessageTargetId)?;
    let user = data
        .resolved
        .as_ref()
        .ok_or(Error::NoResolvedData)?
        .users
        .get(&msg_id.cast())
        .ok_or(Error::NoTarget)?;
    crate::levels::get_level(
        data.guild_id.ok_or(Error::NoGuildId)?,
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
) -> Result<InteractionResponse, Error> {
    let msg_id = data.target_id.ok_or(Error::NoMessageTargetId)?;
    let user = &data
        .resolved
        .as_ref()
        .ok_or(Error::NoResolvedData)?
        .messages
        .get(&msg_id.cast())
        .ok_or(Error::NoTarget)?
        .author;
    crate::levels::get_level(
        data.guild_id.ok_or(Error::NoGuildId)?,
        user.clone(),
        invoker,
        token,
        state,
    )
    .await
}
