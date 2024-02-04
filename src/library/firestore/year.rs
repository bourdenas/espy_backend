use tracing::instrument;

use crate::{api::FirestoreApi, documents::TimelineLegacy, Status};

#[instrument(name = "year::write", level = "trace", skip(firestore))]
pub async fn write(
    firestore: &FirestoreApi,
    timeline: &TimelineLegacy,
    year: u64,
) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col("espy")
        .document_id(format!("{year}"))
        .object(timeline)
        .execute()
        .await?;
    Ok(())
}
