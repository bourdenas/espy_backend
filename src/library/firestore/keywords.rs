use tracing::instrument;

use crate::{api::FirestoreApi, documents::Keyword, Status};

use super::{utils, BatchReadResult};

#[instrument(name = "keywords::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Keyword, Status> {
    utils::read(firestore, KEYWORDS, doc_id.to_string()).await
}

#[instrument(name = "keywords::batch_read", level = "trace", skip(firestore))]
pub async fn batch_read(
    firestore: &FirestoreApi,
    doc_ids: &[u64],
) -> Result<BatchReadResult<Keyword>, Status> {
    utils::batch_read(firestore, KEYWORDS, doc_ids).await
}

#[instrument(name = "keywords::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, keyword: &Keyword) -> Result<(), Status> {
    utils::write(firestore, KEYWORDS, keyword.id.to_string(), keyword).await
}

const KEYWORDS: &str = "keywords";
