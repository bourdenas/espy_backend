use std::collections::HashMap;

use crate::{
    api::FirestoreApi,
    documents::{ExternalGame, StoreEntry},
    Status,
};
use firestore::{path, FirestoreResult};
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use tracing::{instrument, warn};

#[instrument(name = "external_games::read", level = "trace", skip(firestore))]
pub async fn read(
    firestore: &FirestoreApi,
    store: &str,
    store_id: &str,
) -> Result<ExternalGame, Status> {
    let doc_id = format!("{}_{}", store, store_id);

    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(EXTERNAL_GAMES)
        .obj()
        .one(&doc_id)
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{EXTERNAL_GAMES}/{doc_id}' was not found"
        ))),
    }
}

/// Batch reads external games by id.
///
/// Returns a tuple with two vectors. The first one contains the found
/// ExternalGame docs and the second contains the StoreEntry docs that were not
/// found.
#[instrument(
    name = "external_games::batch_read",
    level = "trace",
    skip(firestore, store_entries)
)]
pub async fn batch_read(
    firestore: &FirestoreApi,
    store_entries: Vec<StoreEntry>,
) -> Result<Vec<ExternalGameResult>, Status> {
    let mut store_entries = HashMap::<String, StoreEntry>::from_iter(
        store_entries
            .into_iter()
            .map(|e| (format!("{}_{}", &e.storefront_name, &e.id), e)),
    );

    let mut docs: BoxStream<FirestoreResult<(String, Option<ExternalGame>)>> = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(EXTERNAL_GAMES)
        .obj()
        .batch_with_errors(store_entries.keys())
        .await?;

    let mut results = vec![];
    while let Some(external_game) = docs.next().await {
        match external_game {
            Ok((id, external_game)) => results.push(ExternalGameResult {
                store_entry: store_entries.remove(&id).unwrap_or_default(),
                external_game,
            }),
            Err(status) => warn!("{status}"),
        }
    }

    Ok(results)
}

#[derive(Debug, Clone)]
pub struct ExternalGameResult {
    pub store_entry: StoreEntry,
    pub external_game: Option<ExternalGame>,
}

#[instrument(
    name = "external_games::write",
    level = "trace",
    skip(firestore, external_game)
    fields(
        store_id = %external_game.store_id,
    )
)]
pub async fn write(firestore: &FirestoreApi, external_game: &ExternalGame) -> Result<(), Status> {
    let doc_id = format!("{}_{}", &external_game.store_name, &external_game.store_id);

    firestore
        .db()
        .fluent()
        .update()
        .in_col(EXTERNAL_GAMES)
        .document_id(doc_id)
        .object(external_game)
        .execute()
        .await?;
    Ok(())
}

#[instrument(name = "external_games::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, store: &str, store_id: &str) -> Result<(), Status> {
    let doc_id = format!("{}_{}", store, store_id);

    firestore
        .db()
        .fluent()
        .delete()
        .from(EXTERNAL_GAMES)
        .document_id(doc_id)
        .execute()
        .await?;
    Ok(())
}

pub async fn get_steam_id(firestore: &FirestoreApi, igdb_id: u64) -> Result<String, Status> {
    let external_games: BoxStream<FirestoreResult<ExternalGame>> = firestore
        .db()
        .fluent()
        .select()
        .from("external_games")
        .filter(|q| {
            q.for_all([
                q.field(path!(ExternalGame::igdb_id)).equal(igdb_id),
                q.field(path!(ExternalGame::store_name)).equal("steam"),
            ])
        })
        .obj()
        .stream_query_with_errors()
        .await?;

    let external_games = external_games.try_collect::<Vec<ExternalGame>>().await?;
    match external_games.is_empty() {
        false => Ok(external_games[0].store_id.clone()),
        true => Err(Status::not_found(format!(
            "Steam Id for {igdb_id} was not found"
        ))),
    }
}

const EXTERNAL_GAMES: &str = "external_games";
