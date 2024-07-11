use tracing::instrument;

use crate::{api::FirestoreApi, documents::Collection, Status};

use super::{utils, BatchReadResult};

#[instrument(name = "franchises::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Collection, Status> {
    utils::read(firestore, FRANCHISES, doc_id.to_string()).await
}

/// Batch reads franchises by id.
///
/// Returns a tuple with two vectors. The first one contains the found Franchice
/// docs and the second contains the doc ids that were not found.
#[instrument(
    name = "franchices::batch_read",
    level = "trace",
    skip(firestore, doc_ids)
)]
pub async fn batch_read(
    firestore: &FirestoreApi,
    doc_ids: &[u64],
) -> Result<BatchReadResult<Collection>, Status> {
    utils::batch_read(firestore, FRANCHISES, doc_ids).await
}

#[instrument(
    name = "franchises::write",
    level = "trace",
    skip(firestore, franchise)
    fields(
        franchise = %franchise.slug,
    )
)]
pub async fn write(firestore: &FirestoreApi, franchise: &Collection) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(FRANCHISES)
        .document_id(franchise.id.to_string())
        .object(franchise)
        .execute()
        .await?;
    Ok(())
}

const FRANCHISES: &str = "franchises";
