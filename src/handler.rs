use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse, Json};
use twilight_model::{
    application::interaction::Interaction,
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::AppState;

pub async fn handle(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<(), Error> {
    let body = body.to_vec();
    let interaction: Interaction = serde_json::from_slice(&body)?;
    let my_id = state.my_id;
    let client = state.client.clone();
    let interaction_token = interaction.token.clone();
    let interaction_id = interaction.id.clone();
    let response = match crate::processor::process(interaction, state).await {
        Ok(val) => val,
        Err(e) => {
            error!("{e}");
            InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(
                    InteractionResponseDataBuilder::new()
                        .flags(MessageFlags::EPHEMERAL)
                        .content(e.to_string())
                        .build(),
                ),
            }
        }
    };
    client
        .interaction(my_id)
        .create_response(interaction_id, &interaction_token, &response)
        .await?;
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("serde_json validation error: {0}")]
    Http(#[from] twilight_http::Error),
    #[error("serde_json validation error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}
