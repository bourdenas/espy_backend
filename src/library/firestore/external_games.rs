use std::collections::HashMap;

use crate::{
    api::FirestoreApi,
    documents::{ExternalGame, StoreEntry},
    Status,
};
use firestore::{path, FirestoreResult};
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use tracing::{instrument, warn};

use super::utils;

#[instrument(name = "external_games::read", level = "trace", skip(firestore))]
pub async fn read(
    firestore: &FirestoreApi,
    store: &str,
    store_id: &str,
) -> Result<ExternalGame, Status> {
    let doc_id = format!("{}_{}", store, store_id);
    utils::read(firestore, EXTERNAL_GAMES, doc_id).await
}

/// Batch reads external games based on StoreEntries.
#[instrument(
    name = "external_games::batch_read",
    level = "trace",
    skip(firestore, store_entries)
)]
pub async fn batch_read(
    firestore: &FirestoreApi,
    store_entries: Vec<StoreEntry>,
) -> Result<ExternalGameResult, Status> {
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

    let mut matches = vec![];
    let mut missing = vec![];
    while let Some(external_game) = docs.next().await {
        match external_game {
            Ok((id, external_game)) => match external_game {
                Some(external_game) => matches.push(ExternalMatch {
                    store_entry: store_entries.remove(&id).unwrap_or_default(),
                    external_game,
                }),
                None => missing.push(store_entries.remove(&id).unwrap_or_default()),
            },
            Err(e) => warn!(
                "{}",
                utils::make_status(e, "external_games::batch_read", "?")
            ),
        }
    }

    Ok(ExternalGameResult { matches, missing })
}

#[derive(Debug, Clone)]
pub struct ExternalGameResult {
    pub matches: Vec<ExternalMatch>,
    pub missing: Vec<StoreEntry>,
}

#[derive(Debug, Clone)]
pub struct ExternalMatch {
    pub store_entry: StoreEntry,
    pub external_game: ExternalGame,
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

pub async fn get_external_games(
    firestore: &FirestoreApi,
    igdb_id: u64,
) -> Result<Vec<ExternalGame>, Status> {
    let external_games: BoxStream<FirestoreResult<ExternalGame>> = firestore
        .db()
        .fluent()
        .select()
        .from("external_games")
        .filter(|q| q.for_all([q.field(path!(ExternalGame::igdb_id)).equal(igdb_id)]))
        .obj()
        .stream_query_with_errors()
        .await?;

    Ok(external_games.try_collect::<Vec<ExternalGame>>().await?)
}

const EXTERNAL_GAMES: &str = "external_games";
