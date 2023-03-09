use rand::Rng;
use twilight_model::{
    gateway::payload::incoming::MessageCreate,
    id::{marker::RoleMarker, Id},
};

use crate::AppState;

pub async fn save(msg: MessageCreate, state: AppState) -> Result<(), crate::Error> {
    let Some(guild_id) = msg.guild_id else { return Ok(()); };
    // We ignore cooldown users and bots
    if msg.author.bot || state.cooldowns.contains(guild_id, msg.author.id) {
        return Ok(());
    }
    let xp_count: i64 = rand::thread_rng().gen_range(15..=25);
    // this query is pretty nice. it handles most of the update logic for us. Pretty slow, though- ~100ms total.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    let xp = query!(
        "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3) ON CONFLICT (id, guild)
         DO UPDATE SET xp=levels.xp+excluded.xp RETURNING xp",
        msg.author.id.get() as i64,
        xp_count,
        guild_id.get() as i64
    )
    .fetch_one(&state.db)
    .await?
    .xp as u64;
    // once you're in the DB with no errors, cooldown it.
    state.cooldowns.add(guild_id, msg.author.id).await;
    let level_info = mee6::LevelInfo::new(xp);
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
    let reward = query!(
        "SELECT id FROM role_rewards
            WHERE guild = $1 AND requirement <= $2
            ORDER BY requirement DESC LIMIT 1",
        guild_id.get() as i64,
        level_info.level() as i64
    )
    .fetch_optional(&state.db)
    .await?
    .map(|v| Id::<RoleMarker>::new(v.id as u64));
    if let Some(reward) = reward {
        if let Some(member) = &msg.member {
            if member.roles.contains(&reward) {
                return Ok(());
            }
        }
        state
            .client
            .add_guild_member_role(guild_id, msg.author.id, reward)
            .await?;
    }
    Ok(())
}
