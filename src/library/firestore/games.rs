use std::time::{SystemTime, UNIX_EPOCH};

use futures::{stream::BoxStream, StreamExt};
use tracing::instrument;

use crate::{api::FirestoreApi, documents::GameEntry, Status};

use super::{utils, BatchReadResult};

#[instrument(name = "games::list", level = "trace", skip(firestore))]
pub async fn list(firestore: &FirestoreApi) -> Result<Vec<GameEntry>, Status> {
    let doc_stream: BoxStream<GameEntry> = firestore
        .db()
        .fluent()
        .list()
        .from(GAMES)
        .obj()
        .stream_all()
        .await?;

    Ok(doc_stream.collect().await)
}

#[instrument(name = "games::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<GameEntry, Status> {
    utils::read(firestore, GAMES, doc_id.to_string()).await
}

/// Batch reads games by id.
///
/// Returns a tuple with two vectors. The first one contains the found GameEntry
/// docs and the second contains the doc ids that were not found.
#[instrument(name = "games::batch_read", level = "trace", skip(firestore, doc_ids))]
pub async fn batch_read(
    firestore: &FirestoreApi,
    doc_ids: &[u64],
) -> Result<BatchReadResult<GameEntry>, Status> {
    utils::batch_read(firestore, GAMES, doc_ids).await
}

#[instrument(name = "games::write", level = "trace", skip(firestore, game_entry))]
pub async fn write(firestore: &FirestoreApi, game_entry: &mut GameEntry) -> Result<(), Status> {
    game_entry.last_updated = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    utils::write(firestore, GAMES, game_entry.id.to_string(), game_entry).await
}

#[instrument(name = "games::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, doc_id: u64) -> Result<(), Status> {
    utils::delete(firestore, GAMES, doc_id.to_string()).await
}

const GAMES: &str = "games";
