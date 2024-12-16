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
)]
pub async fn write(firestore: &FirestoreApi, collection: &Collection) -> Result<(), Status> {
    utils::write(
        firestore,
        COLLECTIONS,
        collection.id.to_string(),
        collection,
    )
    .await
}

#[instrument(name = "collections::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, doc_id: u64) -> Result<(), Status> {
    utils::delete(firestore, COLLECTIONS, doc_id.to_string()).await
}

const COLLECTIONS: &str = "collections";
