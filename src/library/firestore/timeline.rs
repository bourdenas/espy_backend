use tracing::instrument;

use crate::{api::FirestoreApi, documents::Timeline, Status};

#[instrument(name = "timeline::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, timeline: &Timeline) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col("espy")
        .document_id("timeline")
        .object(timeline)
        .execute::<()>()
        .await?;
    Ok(())
}
