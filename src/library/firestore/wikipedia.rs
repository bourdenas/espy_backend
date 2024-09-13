use tracing::instrument;

use crate::{api::FirestoreApi, documents::WikipediaData, Status};

use super::utils;

#[instrument(name = "wikipedia::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<WikipediaData, Status> {
    utils::read(firestore, WIKIPEDIA, doc_id.to_string()).await
}

#[instrument(name = "wikipedia::write", level = "trace", skip(firestore, wiki_data))]
pub async fn write(
    firestore: &FirestoreApi,
    id: u64,
    wiki_data: &WikipediaData,
) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(WIKIPEDIA)
        .document_id(id.to_string())
        .object(wiki_data)
        .execute()
        .await?;
    Ok(())
}

const WIKIPEDIA: &str = "wikipedia";
