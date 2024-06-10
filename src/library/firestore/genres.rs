use tracing::instrument;

use crate::{
    api::FirestoreApi,
    documents::{GameEntry, Genre},
    Status,
};

#[instrument(name = "genres::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Option<Genre>, Status> {
    Ok(firestore
        .db()
        .fluent()
        .select()
        .by_id_in(GENRES)
        .obj()
        .one(doc_id.to_string())
        .await?)
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

#[instrument(name = "genres::needs_annotation", level = "trace", skip(firestore))]
pub async fn needs_annotation(
    firestore: &FirestoreApi,
    game_entry: &GameEntry,
) -> Result<(), Status> {
    let clone = GameEntry {
        id: game_entry.id,
        name: game_entry.name.clone(),
        ..Default::default()
    };

    firestore
        .db()
        .fluent()
        .update()
        .in_col(NEEDS_ANNOTATION)
        .document_id(game_entry.id.to_string())
        .object(&clone)
        .execute()
        .await?;
    Ok(())
}

const GENRES: &str = "genres";
const NEEDS_ANNOTATION: &str = "needs_annotation";
