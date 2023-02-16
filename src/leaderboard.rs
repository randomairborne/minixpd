use crate::{cmd_defs::LeaderboardCommand, AppState, Error};

use twilight_model::{
    application::interaction::message_component::MessageComponentInteractionData,
    channel::{
        message::{
            component::{ActionRow, Button, ButtonStyle},
            Component, Embed, MessageFlags, ReactionType,
        },
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{
        marker::{GuildMarker, InteractionMarker, UserMarker},
        Id,
    },
};
use twilight_util::builder::{
    embed::{EmbedBuilder, EmbedFooterBuilder},
    InteractionResponseDataBuilder,
};

pub async fn leaderboard(
    guild_id: Id<GuildMarker>,
    interaction: Id<InteractionMarker>,
    token: String,
    state: AppState,
    prefs: LeaderboardCommand,
) -> Result<InteractionResponse, Error> {
    state.tokens.set(interaction, token).await;
    let zpage = if let Some(pick) = prefs.page {
        pick - 1
    } else if let Some(pick) = prefs.user {
        get_user_position(pick.resolved.id, guild_id, &state.db).await?
    } else {
        0
    };
    let (embed, components) = gen_leaderboard(guild_id, state.db, zpage).await?;
    let data = InteractionResponseDataBuilder::new()
        .embeds([embed])
        .components([Component::ActionRow(ActionRow { components })])
        .flags(MessageFlags::EPHEMERAL)
        .build();

    Ok(InteractionResponse {
        data: Some(data),
        kind: InteractionResponseType::ChannelMessageWithSource,
    })
}

async fn gen_leaderboard(
    guild_id: Id<GuildMarker>,
    db: sqlx::PgPool,
    zpage: i64,
) -> Result<(Embed, Vec<Component>), Error> {
    #[allow(clippy::cast_possible_wrap)]
    let users = query!(
        "SELECT * FROM levels WHERE guild = $1 ORDER BY xp LIMIT 10 OFFSET $2",
        guild_id.get() as i64,
        zpage * 10
    )
    .fetch_all(&db)
    .await?;
    let mut description = String::with_capacity(users.len() * 128);
    #[allow(clippy::cast_sign_loss)]
    for (i, user) in users.iter().enumerate() {
        let rank: i64 = i as i64 + (zpage * 10) + 1;
        description += &format!("{rank}. <@{}>\n", user.id as u64);
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
        custom_id: Some((zpage).to_string()),
        disabled: zpage == 0,
        emoji: Some(ReactionType::Unicode {
            name: "⬅".to_string(),
        }),
        label: Some("Previous".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    let forward_button = Component::Button(Button {
        custom_id: Some((zpage + 2).to_string()),
        disabled: users.len() < 10,
        emoji: Some(ReactionType::Unicode {
            name: "➡️".to_string(),
        }),
        label: Some("Next".to_string()),
        style: ButtonStyle::Primary,
        url: None,
    });
    Ok((embed, vec![back_button, forward_button]))
}

pub async fn process_message_component(
    data: MessageComponentInteractionData,
    guild_id: Id<GuildMarker>,
    interaction: Id<InteractionMarker>,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    let offset = data.custom_id.parse()?;
    #[allow(clippy::cast_possible_wrap)]
    let (embed, components) = gen_leaderboard(guild_id, state.db, offset).await?;
    state
        .client
        .interaction(state.my_id)
        .update_response(&state.tokens.get(interaction).ok_or(Error::LeaderboardExpired)?)
        .components(Some(&[Component::ActionRow(ActionRow { components })]))?
        .embeds(Some(&[embed]))?
        .await?;
    Ok(InteractionResponse {
        kind: InteractionResponseType::Pong,
        data: None,
    })
}

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
