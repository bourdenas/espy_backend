use tracing::instrument;

use crate::{api::FirestoreApi, documents::Timeline, Status};

use super::utils;

#[instrument(name = "timeline::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, timeline: &Timeline) -> Result<(), Status> {
    utils::write(firestore, "espy", "timeline".to_owned(), timeline).await
}
