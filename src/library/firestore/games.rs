use std::time::{SystemTime, UNIX_EPOCH};

use firestore::FirestoreResult;
use futures::{stream::BoxStream, StreamExt};
use tracing::{instrument, warn};

use crate::{api::FirestoreApi, documents::GameEntry, Status};

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
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(GAMES)
        .obj()
        .one(doc_id.to_string())
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{GAMES}/{doc_id}' was not found"
        ))),
    }
}

#[instrument(name = "games::batch_read", level = "trace", skip(firestore))]
pub async fn batch_read(
    firestore: &FirestoreApi,
    doc_ids: &[u64],
) -> Result<Vec<GameEntry>, Status> {
    let mut docs: BoxStream<FirestoreResult<(String, Option<GameEntry>)>> = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(GAMES)
        .obj()
        .batch_with_errors(doc_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>())
        .await?;

    let mut games: Vec<GameEntry> = vec![];
    while let Some(game) = docs.next().await {
        match game {
            Ok((_, game)) => match game {
                Some(game) => games.push(game),
                None => {}
            },
            Err(status) => warn!("{status}"),
        }
    }

    Ok(games)
}

#[instrument(name = "games::write", level = "trace", skip(firestore, game_entry))]
pub async fn write(firestore: &FirestoreApi, game_entry: &mut GameEntry) -> Result<(), Status> {
    game_entry.last_updated = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    firestore
        .db()
        .fluent()
        .update()
        .in_col(GAMES)
        .document_id(game_entry.id.to_string())
        .object(game_entry)
        .execute()
        .await?;
    Ok(())
}

#[instrument(name = "games::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, doc_id: u64) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .delete()
        .from(GAMES)
        .document_id(doc_id.to_string())
        .execute()
        .await?;
    Ok(())
}

const GAMES: &str = "games";
