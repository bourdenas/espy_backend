use crate::Status;
use tracing::instrument;

use super::{
    backend::create_webhook,
    resolve::{EXTERNAL_GAMES_ENDPOINT, GAMES_ENDPOINT, GENRES_ENDPOINT, KEYWORDS_ENDPOINT},
    IgdbConnection,
};

pub struct IgdbWebhooksApi {
    connection: IgdbConnection,
}

impl IgdbWebhooksApi {
    pub fn new(connection: IgdbConnection) -> IgdbWebhooksApi {
        IgdbWebhooksApi { connection }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn register_games_webhook(
        &self,
        webhook_url: &str,
        secret: &str,
    ) -> Result<(), Status> {
        create_webhook(
            &self.connection,
            GAMES_ENDPOINT,
            &format!("{webhook_url}/add_game"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            GAMES_ENDPOINT,
            &format!("{webhook_url}/update_game"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            EXTERNAL_GAMES_ENDPOINT,
            &format!("{webhook_url}/external_games"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            EXTERNAL_GAMES_ENDPOINT,
            &format!("{webhook_url}/external_games"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            GENRES_ENDPOINT,
            &format!("{webhook_url}/genres"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            GENRES_ENDPOINT,
            &format!("{webhook_url}/genres"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            KEYWORDS_ENDPOINT,
            &format!("{webhook_url}/keywords"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            KEYWORDS_ENDPOINT,
            &format!("{webhook_url}/keywords"),
            "update",
            secret,
        )
        .await
    }
}
