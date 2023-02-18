use crate::{cmd_defs::LeaderboardCommand, AppState, Error};

use std::fmt::Write;
use twilight_model::{
    application::interaction::message_component::MessageComponentInteractionData,
    channel::message::{
        component::{ActionRow, Button, ButtonStyle},
        Component, ReactionType,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::{
    embed::{EmbedBuilder, EmbedFooterBuilder},
    InteractionResponseDataBuilder,
};

pub async fn leaderboard(
    guild_id: Id<GuildMarker>,
    state: AppState,
    prefs: LeaderboardCommand,
) -> Result<InteractionResponse, Error> {
    // "zpage" means "zero-indexed page", which is how this is represented internally.
    // We add one whenever we show it to the user, and add one every time we get it from the user.
    let zpage = if let Some(pick) = prefs.page {
        pick - 1
    } else if let Some(pick) = prefs.user {
        get_user_position(pick.resolved.id, guild_id, &state.db).await?
    } else {
        0
    };
    Ok(InteractionResponse {
        data: Some(gen_leaderboard(guild_id, state.db, zpage).await?),
        kind: InteractionResponseType::ChannelMessageWithSource,
    })
}

async fn gen_leaderboard(
    guild_id: Id<GuildMarker>,
    db: sqlx::PgPool,
    zpage: i64,
) -> Result<InteractionResponseData, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let users = query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp DESC LIMIT 10 OFFSET $2",
        guild_id.get() as i64,
        zpage * 10
    )
    .fetch_all(&db)
    .await?;
    // this is kinda the only way to do this
    // It's designed to only allocate once, at the start here
    let mut description = String::with_capacity(users.len() * 128);
    #[allow(clippy::cast_sign_loss)]
    for (i, user) in users.iter().enumerate() {
        let level = mee6::LevelInfo::new(user.xp as u64).level();
        let rank: i64 = i as i64 + (zpage * 10) + 1;
        writeln!(description, "Level {level}. <@{}> #{rank}", user.id as u64).ok();
    }
    if description.is_empty() {
        description += "Nobody is ranked yet.";
    }
    let embed = EmbedBuilder::new()
        .description(description)
        .footer(EmbedFooterBuilder::new(format!("Page {}", zpage + 1)).build())
        .color(crate::THEME_COLOR)
        .build();
    let back_button = Component::Button(Button {
        custom_id: Some((zpage - 1).to_string()),
        disabled: zpage == 0,
        emoji: Some(ReactionType::Unicode {
            name: "⬅".to_string(),
        }),
        label: Some("Previous".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    let forward_button = Component::Button(Button {
        custom_id: Some((zpage + 1).to_string()),
        // this checks if the users on the current page are less then 10.
        // If this is the case, that means we *must* be at the last page.
        // this saves us doing weird counting shenanigans with the db
        disabled: users.len() < 10,
        emoji: Some(ReactionType::Unicode {
            name: "➡️".to_string(),
        }),
        label: Some("Next".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    Ok(InteractionResponseDataBuilder::new()
        .components([Component::ActionRow(ActionRow {
            components: vec![back_button, forward_button],
        })])
        .embeds([embed])
        .build())
}

pub async fn process_message_component(
    data: MessageComponentInteractionData,
    guild_id: Id<GuildMarker>,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    // when we create the buttons, we set next and previous's custom IDs to the current page
    // plus and minus 1. This means that we don't have to store which page which
    // message is on, because the component will tell us exactly where it wants to go!
    let offset: i64 = data.custom_id.parse()?;
    Ok(InteractionResponse {
        kind: InteractionResponseType::UpdateMessage,
        data: Some(gen_leaderboard(guild_id, state.db, offset).await?),
    })
}

// this is a really simple wrapper function
async fn get_user_position(
    user_id: Id<UserMarker>,
    guild_id: Id<GuildMarker>,
    db: &sqlx::PgPool,
) -> Result<i64, Error> {
    #[allow(clippy::cast_possible_wrap)]
    Ok(query!(
        "SELECT COUNT(*) as count FROM levels WHERE xp > (SELECT xp FROM levels WHERE id = $1) AND guild = $2",
        user_id.get() as i64,
        guild_id.get() as i64
    )
    .fetch_one(db)
    .await?
    .count.unwrap_or(0))
}
