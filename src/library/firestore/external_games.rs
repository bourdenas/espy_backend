use crate::{api::FirestoreApi, documents::ExternalGame, Status};
use firestore::{path, FirestoreResult};
use futures::{stream::BoxStream, TryStreamExt};
use tracing::instrument;

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
