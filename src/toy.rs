use std::fmt::Display;

use twilight_interactions::command::{CommandOption, CreateOption};
use twilight_model::{
    channel::message::{Embed, MessageFlags},
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
    user::User,
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use crate::{AppState, Error};

pub async fn modify(
    toy: Toy,
    guild_id: Id<GuildMarker>,
    invoker: User,
    state: AppState,
) -> Result<InteractionResponse, Error> {
    #[allow(clippy::cast_possible_wrap)]
    let xp = query!(
        "SELECT * FROM levels WHERE id = $1 AND guild = $2",
        invoker.id.get() as i64,
        guild_id.get() as i64
    )
    .fetch_optional(&state.db)
    .await?
    .map_or(0, |r| r.xp);
    #[allow(clippy::cast_sign_loss)]
    let level_info = mee6::LevelInfo::new(xp as u64);
    if let Some(level_requirement) = toy.level_requirement() {
        if level_info.level() < level_requirement {
            // i break the rules on error handling here. It does make nicer UX.
            let embed = EmbedBuilder::new()
                .description(format!(
                    "You need at least {} levels for {toy} (you have {})",
                    level_requirement,
                    level_info.level()
                ))
                .build();
            return Ok(ephemeral_embed_response(embed));
        }
    }
    #[allow(clippy::cast_sign_loss)]
    if let Some(id_list) = toy.id_requirement() {
        if !id_list.contains(&invoker.id) {
            // i break the rules on error handling here. It does make nicer UX.
            let embed = EmbedBuilder::new()
                .description(
                    "You need to be on the allow-list of the bot to use this icon!".to_string(),
                )
                .build();
            return Ok(ephemeral_embed_response(embed));
        }
    }
    #[allow(clippy::cast_possible_wrap)]
    query!(
        "INSERT INTO card_toy(id, guild_id, toy) VALUES ($1, $2, $3)",
        invoker.id.get() as i64,
        guild_id.get() as i64,
        toy.value()
    )
    .execute(&state.db)
    .await?;
    let embed = EmbedBuilder::new()
        .description(format!("Set your toy to {toy}!"))
        .build();
    Ok(ephemeral_embed_response(embed))
}

#[derive(Clone, Copy, Debug, CreateOption, CommandOption)]
pub enum Toy {
    #[option(name = "Fox", value = "fox.png")]
    Fox,
    #[option(name = "Parrot", value = "parrot.png")]
    Parrot,
    #[option(name = "Grass Block", value = "grassblock.png")]
    GrassBlock,
    #[option(name = "Pickaxe", value = "pickaxe.png")]
    Pickaxe,
    #[option(name = "Steve heart", value = "steveheart.png")]
    SteveHeart,
    #[option(name = "Tree", value = "tree.png")]
    Tree,
    #[option(name = "Airplane", value = "airplane.png")]
    Airplane,
}

impl Toy {
    pub const fn level_requirement(self) -> Option<u64> {
        match self {
            Self::Pickaxe => Some(10),
            Self::Fox | Self::Parrot => Some(5),
            _ => None,
        }
    }
    pub fn id_requirement(self) -> Option<Vec<Id<UserMarker>>> {
        match self {
            Self::Airplane => Some(vec![
                Id::new(788_222_689_126_776_832),
                Id::new(526_092_507_965_161_474),
            ]),
            _ => None,
        }
    }
}

impl Display for Toy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Fox => "Fox",
            Self::Parrot => "Parrot",
            Self::GrassBlock => "Grass block",
            Self::Pickaxe => "Pickaxe",
            Self::SteveHeart => "Steve heart",
            Self::Tree => "Tree",
            Self::Airplane => "Airplane",
        };
        f.write_str(text)
    }
}

fn ephemeral_embed_response(embed: Embed) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .embeds([embed])
                .flags(MessageFlags::EPHEMERAL)
                .build(),
        ),
    }
}
