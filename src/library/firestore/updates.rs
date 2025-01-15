use tracing::instrument;

use crate::{api::FirestoreApi, documents::DayUpdates, Status};

use super::utils;

#[instrument(name = "updates::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, day_updates: &DayUpdates) -> Result<(), Status> {
    utils::write(firestore, "updates", day_updates.date.clone(), day_updates).await
}
