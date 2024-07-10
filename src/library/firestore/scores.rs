use tracing::instrument;

use crate::{api::FirestoreApi, documents::ScoresDoc, Status};

use super::utils;

#[instrument(name = "scores::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<ScoresDoc, Status> {
    utils::read(firestore, SCORES, doc_id.to_string()).await
}

#[instrument(name = "scores::write", level = "trace", skip(firestore, game_entry))]
pub async fn write(firestore: &FirestoreApi, game_entry: &ScoresDoc) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(SCORES)
        .document_id(game_entry.id.to_string())
        .object(game_entry)
        .execute()
        .await?;
    Ok(())
}

const SCORES: &str = "scores";
