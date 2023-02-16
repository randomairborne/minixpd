use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use crate::{AppState, Error};

pub async fn handle(interaction: Interaction, state: AppState) -> Result<(), Error> {
    let interaction_token = interaction.token.clone();
    let interaction_id = interaction.id;
    let response = match crate::processor::process(interaction, state.clone()).await {
        Ok(val) => val,
        Err(e) => {
            error!("{e:#?}");
            let embed = EmbedBuilder::new().description(format!("❌ {e}")).build();
            InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(
                    InteractionResponseDataBuilder::new()
                        .flags(MessageFlags::EPHEMERAL)
                        .embeds([embed])
                        .build(),
                ),
            }
        }
    };
    state
        .client
        .interaction(state.my_id)
        .create_response(interaction_id, &interaction_token, &response)
        .await?;
    Ok(())
}
