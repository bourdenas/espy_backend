use crate::{
    api::FirestoreApi,
    documents::{GameDigest, Library, LibraryEntry},
    Status,
};
use tracing::instrument;

use super::utils;

#[instrument(name = "wishlist::read", level = "trace", skip(firestore, user_id))]
async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<Library, Status> {
    utils::auth_read(firestore, user_id, GAMES, WISHLIST_DOC.to_owned()).await
}

#[instrument(
    name = "wishlist::write",
    level = "trace",
    skip(firestore, user_id, library)
)]
async fn write(
    firestore: &FirestoreApi,
    user_id: &str,
    mut library: Library,
) -> Result<(), Status> {
    library
        .entries
        .sort_by(|l, r| r.digest.release_date.cmp(&l.digest.release_date));
    utils::auth_write(firestore, user_id, GAMES, WISHLIST_DOC.to_owned(), &library).await
}

#[instrument(
    name = "wishlist::add_entry",
    level = "trace",
    skip(firestore, user_id, library_entry),
    fields(
        game_id = %library_entry.id
    ),
)]
pub async fn add_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    library_entry: LibraryEntry,
) -> Result<(), Status> {
    let mut wishlist = read(firestore, user_id).await?;
    if add(library_entry, &mut wishlist) {
        write(firestore, user_id, wishlist).await?;
    }
    Ok(())
}

#[instrument(
    name = "wishlist::remove_entry",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn remove_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    game_id: u64,
) -> Result<(), Status> {
    let mut wishlist = read(firestore, user_id).await?;
    if remove(game_id, &mut wishlist) {
        return write(firestore, user_id, wishlist).await;
    }
    Ok(())
}

#[instrument(
    name = "wishlist::remove_entry",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn remove_entries(
    firestore: &FirestoreApi,
    user_id: &str,
    game_ids: &[u64],
) -> Result<(), Status> {
    let mut wishlist = read(firestore, user_id).await?;

    if game_ids
        .into_iter()
        .fold(false, |dirty, id| dirty || remove(*id, &mut wishlist))
    {
        write(firestore, user_id, wishlist).await?;
    }
    Ok(())
}

#[instrument(
    name = "wishlist::update_entry",
    level = "trace",
    skip(firestore, user_id, game_digest)
)]
pub async fn update_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    game_digest: GameDigest,
) -> Result<(), Status> {
    let mut wishlist = read(firestore, user_id).await?;

    match wishlist.entries.iter_mut().find(|e| e.id == game_digest.id) {
        Some(existing_entry) => existing_entry.digest = game_digest,
        None => {
            return Err(Status::not_found("not in wishlist"));
        }
    }

    write(firestore, user_id, wishlist).await
}

fn add(library_entry: LibraryEntry, wishlist: &mut Library) -> bool {
    match wishlist.entries.iter().find(|e| e.id == library_entry.id) {
        Some(_) => false,
        None => {
            wishlist.entries.push(library_entry);
            true
        }
    }
}

fn remove(game_id: u64, wishlist: &mut Library) -> bool {
    let original_len = wishlist.entries.len();
    wishlist.entries.retain(|e| e.id != game_id);
    wishlist.entries.len() != original_len
}

const GAMES: &str = "games";
const WISHLIST_DOC: &str = "wishlist";
