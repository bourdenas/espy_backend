use crate::{api::FirestoreApi, documents::Collection, Status};
use tracing::instrument;

use super::{utils, BatchReadResult};

#[instrument(name = "collections::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Collection, Status> {
    utils::read(firestore, COLLECTIONS, doc_id.to_string()).await
}

/// Batch reads collections by id.
///
/// Returns a tuple with two vectors. The first one contains the found
/// Collection docs and the second contains the doc ids that were not found.
#[instrument(
    name = "collections::batch_read",
    level = "trace",
    skip(firestore, doc_ids)
)]
pub async fn batch_read(
    firestore: &FirestoreApi,
    doc_ids: &[u64],
) -> Result<BatchReadResult<Collection>, Status> {
    utils::batch_read(firestore, COLLECTIONS, doc_ids).await
}

#[instrument(
    name = "collections::write",
    level = "trace",
    skip(firestore, collection)
    fields(
        collection = %collection.slug,
    )
)]
pub async fn write(firestore: &FirestoreApi, collection: &Collection) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(COLLECTIONS)
        .document_id(collection.id.to_string())
        .object(collection)
        .execute()
        .await?;
    Ok(())
}

#[instrument(name = "collections::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, doc_id: u64) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .delete()
        .from(COLLECTIONS)
        .document_id(doc_id.to_string())
        .execute()
        .await?;
    Ok(())
}

const COLLECTIONS: &str = "collections";
