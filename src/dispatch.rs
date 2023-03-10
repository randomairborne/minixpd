use crate::{AppState, Error};
use twilight_interactions::command::CommandModel;
use twilight_model::{
    application::{
        command::CommandType,
        interaction::{application_command::CommandData, Interaction, InteractionData},
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{marker::GuildMarker, Id},
    user::User,
};

const PONG: InteractionResponse = InteractionResponse {
    kind: InteractionResponseType::Pong,
    data: None,
};

pub async fn process_interaction(
    interaction: Interaction,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    // discord doesn't always send user for some reason. Dumb.
    let invoker = match interaction.member {
        Some(val) => val.user,
        None => interaction.user,
    }
    .ok_or(Error::NoInvoker)?;
    let guild_id = interaction.guild_id.ok_or(Error::NoGuildId)?;
    if let Some(data) = interaction.data {
        let resp = match data {
            // app command == slash command
            InteractionData::ApplicationCommand(ac) => {
                process_app_cmd(*ac, interaction.token, guild_id, invoker, state).await?
            }
            // this only applies to the leaderboard. It's the forward and back buttons.
            InteractionData::MessageComponent(mc) => {
                crate::leaderboard::process_message_component(mc, guild_id, state).await?
            }
            InteractionData::ModalSubmit(ms) => {
                crate::leaderboard::process_modal_submit(ms, guild_id, state).await?
            }
            _ => PONG,
        };
        Ok(resp)
    } else {
        Err(Error::NoInteractionData)
    }
}

async fn process_app_cmd(
    data: CommandData,
    token: String,
    guild_id: Id<GuildMarker>,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    match data.kind {
        CommandType::ChatInput => process_slash_cmd(data, token, guild_id, invoker, state).await,
        CommandType::User => process_user_cmd(data, token, invoker, state).await,
        CommandType::Message => process_msg_cmd(data, token, invoker, state).await,
        _ => Err(Error::WrongInteractionData),
    }
}

async fn process_slash_cmd(
    data: CommandData,
    token: String,
    guild_id: Id<GuildMarker>,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    match data.name.as_str() {
        "rank" => {
            let target = crate::cmd_defs::RankCommand::from_interaction(data.into())?
                .user
                .map_or_else(|| invoker.clone(), |v| v.resolved);
            crate::levels::get_level(guild_id, target, invoker, token, state).await
        }
        "leaderboard" => {
            let prefs = crate::cmd_defs::LeaderboardCommand::from_interaction(data.into())?;
            crate::leaderboard::leaderboard(guild_id, state, prefs).await
        }
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
