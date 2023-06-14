use crate::Status;
use tracing::instrument;

use super::{backend::create_webhook, resolve::GAMES_ENDPOINT, IgdbApi};

pub struct IgdbWebhooksApi {
    service: IgdbApi,
}

impl IgdbWebhooksApi {
    pub fn new(service: IgdbApi) -> IgdbWebhooksApi {
        IgdbWebhooksApi { service }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn register_games_webhook(
        &self,
        webhook_url: &str,
        secret: &str,
    ) -> Result<(), Status> {
        let connection = self.service.connection()?;
        create_webhook(
            &connection,
            GAMES_ENDPOINT,
            &format!("{webhook_url}/add_game"),
            "create",
            secret,
        )
        .await?;
        create_webhook(
            &connection,
            GAMES_ENDPOINT,
            &format!("{webhook_url}/update_game"),
            "update",
            secret,
        )
        .await
    }
}
