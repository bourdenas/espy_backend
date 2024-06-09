use tracing::instrument;

use crate::{api::FirestoreApi, documents::Genre, Status};

#[instrument(name = "genres::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Genre, Status> {
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(GENRES)
        .obj()
        .one(doc_id.to_string())
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{GENRES}/{doc_id}' was not found"
        ))),
    }
}

#[instrument(name = "genres::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, genre: &Genre) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(GENRES)
        .document_id(genre.game_id.to_string())
        .object(genre)
        .execute()
        .await?;
    Ok(())
}

const GENRES: &str = "genres";
