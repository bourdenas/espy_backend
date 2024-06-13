use tracing::instrument;

use crate::{api::FirestoreApi, documents::ScoresDoc, Status};

#[instrument(name = "games::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<ScoresDoc, Status> {
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(SCORES)
        .obj()
        .one(doc_id.to_string())
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{SCORES}/{doc_id}' was not found"
        ))),
    }
}

#[instrument(name = "games::write", level = "trace", skip(firestore, game_entry))]
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
