use std::collections::HashMap;

use crate::{
    api::FirestoreApi,
    documents::{ExternalGame, StoreEntry},
    logging::FirestoreEvent,
    Status,
};
use firestore::path;
use futures::StreamExt;
use tracing::instrument;

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

/// Batch reads external games in StoreEntries and returns matche `StoreEntry`
/// with `ExternalGame`.
///
/// NOTE: It re-implements the logic of utils::batch because it returns
/// `ExternalMatch` instead of the collection's document type.
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

    let result = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(EXTERNAL_GAMES)
        .obj()
        .batch_with_errors(store_entries.keys())
        .await;

    match result {
        Ok(mut stream) => {
            let mut matches = vec![];
            let mut not_found = vec![];
            let mut errors = vec![];
            while let Some(external_game) = stream.next().await {
                match external_game {
                    Ok((id, external_game)) => match external_game {
                        Some(external_game) => matches.push(ExternalMatch {
                            store_entry: store_entries.remove(&id).unwrap_or_default(),
                            external_game,
                        }),
                        None => not_found.push(store_entries.remove(&id).unwrap_or_default()),
                    },
                    Err(e) => errors.push(e.to_string()),
                }
            }

            FirestoreEvent::search(matches.len(), not_found.len(), errors);
            Ok(ExternalGameResult { matches, not_found })
        }
        Err(e) => {
            FirestoreEvent::search(0, 1, vec![e.to_string()]);
            Err(utils::make_status(
                e,
                EXTERNAL_GAMES,
                format!("by_id #{}", store_entries.len()),
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExternalGameResult {
    pub matches: Vec<ExternalMatch>,
    pub not_found: Vec<StoreEntry>,
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
)]
pub async fn write(firestore: &FirestoreApi, external_game: &ExternalGame) -> Result<(), Status> {
    let doc_id = format!("{}_{}", &external_game.store_name, &external_game.store_id);
    utils::write(firestore, EXTERNAL_GAMES, doc_id, external_game).await
}

#[instrument(name = "external_games::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, store: &str, store_id: &str) -> Result<(), Status> {
    let doc_id = format!("{}_{}", store, store_id);
    utils::delete(firestore, EXTERNAL_GAMES, doc_id).await
}

pub async fn get_steam_id(
    firestore: &FirestoreApi,
    igdb_id: u64,
) -> Result<Option<String>, Status> {
    let result = firestore
        .db()
        .fluent()
        .select()
        .from(EXTERNAL_GAMES)
        .filter(|q| {
            q.for_all([
                q.field(path!(ExternalGame::igdb_id)).equal(igdb_id),
                q.field(path!(ExternalGame::store_name)).equal("steam"),
            ])
        })
        .obj()
        .stream_query_with_errors()
        .await;

    match result {
        Ok(mut stream) => {
            let mut external_games: Vec<ExternalGame> = vec![];
            let mut errors = vec![];
            while let Some(external) = stream.next().await {
                match external {
                    Ok(external) => external_games.push(external),
                    Err(e) => errors.push(e.to_string()),
                }
            }

            FirestoreEvent::search(external_games.len(), errors.len(), errors);

            Ok(match external_games.is_empty() {
                false => Some(external_games[0].store_id.clone()),
                true => None,
            })
        }
        Err(e) => {
            FirestoreEvent::search(0, 1, vec![e.to_string()]);
            Err(utils::make_status(
                e,
                EXTERNAL_GAMES,
                format!("igdb_id={} && store_name=steam", igdb_id),
            ))
        }
    }
}

pub async fn get_external_games(
    firestore: &FirestoreApi,
    igdb_id: u64,
) -> Result<Vec<ExternalGame>, Status> {
    let result = firestore
        .db()
        .fluent()
        .select()
        .from("external_games")
        .filter(|q| q.for_all([q.field(path!(ExternalGame::igdb_id)).equal(igdb_id)]))
        .obj()
        .stream_query_with_errors()
        .await;

    match result {
        Ok(mut stream) => {
            let mut external_games: Vec<ExternalGame> = vec![];
            let mut errors = vec![];
            while let Some(external) = stream.next().await {
                match external {
                    Ok(external) => external_games.push(external),
                    Err(e) => errors.push(e.to_string()),
                }
            }

            FirestoreEvent::search(external_games.len(), errors.len(), errors);
            Ok(external_games)
        }
        Err(e) => {
            FirestoreEvent::search(0, 1, vec![e.to_string()]);
            Err(utils::make_status(
                e,
                EXTERNAL_GAMES,
                format!("igdb_id={}", igdb_id),
            ))
        }
    }
}

const EXTERNAL_GAMES: &str = "external_games";
