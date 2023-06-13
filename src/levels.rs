use crate::{AppState, Error};

use base64::Engine;
use twilight_model::{
    http::{
        attachment::Attachment,
        interaction::{InteractionResponse, InteractionResponseType},
    },
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

pub async fn get_level(
    guild_id: Id<GuildMarker>,
    user: User,
    invoker: User,
    token: String,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let guild_id = guild_id.get() as i64;
    // Select current XP from the database, return 0 if there is no row
    #[allow(clippy::cast_possible_wrap)]
    let xp = query!(
        "SELECT xp FROM levels WHERE id = $1 AND guild = $2",
        user.id.get() as i64,
        guild_id
    )
    .fetch_optional(&state.db)
    .await?
    .map_or(0, |v| v.xp);
    let rank = query!(
        "SELECT COUNT(*) as count FROM levels WHERE xp > $1 AND guild = $2",
        xp,
        guild_id
    )
    .fetch_one(&state.db)
    .await?
    .count
    .unwrap_or(0)
        + 1;
    #[allow(clippy::cast_sign_loss)]
    let level_info = mee6::LevelInfo::new(xp as u64);
    // I am really not a big fan of this. Too much nesting. However, as far as i can tell
    // it does get the parts of speech right.
    let content = if user.bot {
        "Bots aren't ranked, that would be silly!".to_string()
    } else if invoker == user {
        if xp == 0 {
            "You aren't ranked yet, because you haven't sent any messages!".to_string()
        } else {
            return generate_level_response(state, token, user, level_info, rank).await;
        }
    } else if xp == 0 {
        format!(
            "{}#{} isn't ranked yet, because they haven't sent any messages!",
            user.name,
            user.discriminator()
        )
    } else {
        return generate_level_response(state, token, user, level_info, rank).await;
    };
    Ok(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .embeds([EmbedBuilder::new().description(content).build()])
                .build(),
        ),
    })
}

async fn generate_level_response(
    state: AppState,
    token: String,
    user: User,
    level_info: mee6::LevelInfo,
    rank: i64,
) -> Result<InteractionResponse, Error> {
    tokio::task::spawn(async move {
        let Err(err) = add_card(state.clone(), &token, user, level_info, rank).await else { return; };
        let interaction_client = state.client.interaction(state.my_id);
        let embed = EmbedBuilder::new().description(err.to_string()).build();
        let embeds = &[embed];
        match interaction_client.create_followup(&token).embeds(embeds) {
            Ok(awaitable) => {
                if let Err(e) = awaitable.await {
                    warn!("{e:#?}");
                };
            }
            Err(e) => {
                warn!("{e:#?}");
            }
        };
    });
    Ok(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

async fn add_card(
    state: AppState,
    token: &str,
    user: User,
    level_info: mee6::LevelInfo,
    rank: i64,
) -> Result<(), Error> {
    let interaction_client = state.client.interaction(state.my_id);
    #[allow(clippy::cast_possible_wrap)]
    let toy = query!(
        "SELECT toy FROM card_toy WHERE id = $1",
        user.id.get() as i64
    )
    .fetch_optional(&state.db)
    .await?
    .and_then(|v| xpd_rank_card::Toy::from_filename(&v.toy));
    let avatar = get_avatar(&state, &user).await?;
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    let discriminator = if user.discriminator == 0 {
        None
    } else {
        Some(user.discriminator().to_string())
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let png = state
        .svg
        .render(xpd_rank_card::Context {
            level: level_info.level(),
            rank,
            name: user.name.clone(),
            discriminator,
            percentage: (level_info.percentage() * 100.0).round() as u64,
            current: level_info.xp(),
            needed: mee6::xp_needed_for_level(level_info.level() + 1),
            toy,
            avatar,
            font: xpd_rank_card::Font::Mojang,
            colors: xpd_rank_card::colors::Colors::default(),
        })
        .await?;
    let card = Attachment {
        description: Some(format!(
            "{}#{} is level {} (rank #{}), and is {}% of the way to level {}.",
            user.name,
            user.discriminator(),
            level_info.level(),
            rank,
            (level_info.percentage() * 100.0).round(),
            level_info.level() + 1
        )),
        file: png,
        filename: "card.png".to_string(),
        id: 0,
    };
    interaction_client
        .create_followup(token)
        .attachments(&[card])?
        .await?;
    Ok(())
}

async fn get_avatar(state: &AppState, user: &User) -> Result<String, Error> {
    let url = user.avatar.map_or_else(
        || {
            format!(
                "https://cdn.discordapp.com/embed/avatars/{}/{}.png",
                user.id,
                user.discriminator % 5
            )
        },
        |hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                user.id, hash
            )
        },
    );
    let png = state.http.get(url).send().await?.bytes().await?;
    let data = format!("data:image/png;base64,{}", BASE64_ENGINE.encode(png));
    Ok(data)
}

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::general_purpose::NO_PAD,
);
