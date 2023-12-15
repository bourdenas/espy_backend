use tracing::instrument;

use crate::{api::FirestoreApi, documents::Timeline, Status};

#[instrument(name = "year::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, timeline: &Timeline, year: u64) -> Result<(), Status> {
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
