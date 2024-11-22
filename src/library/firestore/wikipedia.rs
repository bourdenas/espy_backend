use tracing::instrument;

use crate::{api::FirestoreApi, documents::WikipediaData, Status};

use super::{utils, BatchReadResult};

#[instrument(name = "wikipedia::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<WikipediaData, Status> {
    utils::read(firestore, WIKIPEDIA, doc_id.to_string()).await
}

#[instrument(
    name = "wikipedia::batch_read",
    level = "trace",
    skip(firestore, doc_ids)
)]
pub async fn batch_read(
    firestore: &FirestoreApi,
    doc_ids: &[u64],
) -> Result<BatchReadResult<WikipediaData>, Status> {
    utils::batch_read(firestore, WIKIPEDIA, doc_ids).await
}

#[instrument(name = "wikipedia::write", level = "trace", skip(firestore, wiki_data))]
pub async fn write(
    firestore: &FirestoreApi,
    doc_id: u64,
    wiki_data: &WikipediaData,
) -> Result<(), Status> {
    utils::write(firestore, WIKIPEDIA, doc_id.to_string(), wiki_data).await
}

const WIKIPEDIA: &str = "wikipedia";
