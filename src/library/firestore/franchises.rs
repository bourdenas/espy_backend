use tracing::instrument;

use crate::{api::FirestoreApi, documents::Collection, Status};

use super::{utils, BatchReadResult};

#[instrument(name = "franchises::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Collection, Status> {
    utils::read(firestore, FRANCHISES, doc_id.to_string()).await
}

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
)]
pub async fn write(firestore: &FirestoreApi, franchise: &Collection) -> Result<(), Status> {
    utils::write(firestore, FRANCHISES, franchise.id.to_string(), franchise).await
}

const FRANCHISES: &str = "franchises";
