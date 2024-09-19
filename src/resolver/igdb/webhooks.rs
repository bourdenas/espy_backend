use crate::Status;
use tracing::instrument;

use super::{backend::create_webhook, endpoints, IgdbConnection};

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
            endpoints::GAMES,
            &format!("{webhook_url}/add_game"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::GAMES,
            &format!("{webhook_url}/update_game"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::EXTERNAL_GAMES,
            &format!("{webhook_url}/external_games"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::EXTERNAL_GAMES,
            &format!("{webhook_url}/external_games"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::GENRES,
            &format!("{webhook_url}/genres"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::GENRES,
            &format!("{webhook_url}/genres"),
            "update",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::KEYWORDS,
            &format!("{webhook_url}/keywords"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &self.connection,
            endpoints::KEYWORDS,
            &format!("{webhook_url}/keywords"),
            "update",
            secret,
        )
        .await
    }
}
