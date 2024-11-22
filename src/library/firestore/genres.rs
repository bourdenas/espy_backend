use tracing::instrument;

use crate::{
    api::FirestoreApi,
    documents::{GameEntry, Genre},
    Status,
};

use super::utils;

#[instrument(name = "genres::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Genre, Status> {
    utils::read(firestore, GENRES, doc_id.to_string()).await
}

#[instrument(name = "genres::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, genre: &Genre) -> Result<(), Status> {
    utils::write(firestore, GENRES, genre.game_id.to_string(), genre).await
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

    utils::write(
        firestore,
        NEEDS_ANNOTATION,
        game_entry.id.to_string(),
        &clone,
    )
    .await
}

const GENRES: &str = "genres";
const NEEDS_ANNOTATION: &str = "needs_annotation";
