use crate::{
    api::FirestoreApi,
    documents::{GameDigest, Library, LibraryEntry},
    Status,
};
use tracing::instrument;

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
    let old_size = wishlist.entries.len();
    for game_id in game_ids {
        remove(*game_id, &mut wishlist);
    }

    match wishlist.entries.len() < old_size {
        true => write(firestore, user_id, wishlist).await,
        false => Ok(()),
    }
}

#[instrument(
    name = "wishlist::update_entry",
    level = "trace",
    skip(firestore, user_id, digests)
)]
pub async fn update_entry(
    firestore: &FirestoreApi,
    user_id: &str,
    game_id: u64,
    digests: Vec<GameDigest>,
) -> Result<(), Status> {
    let mut wishlist = read(firestore, user_id).await?;

    for digest in digests {
        match wishlist.entries.iter_mut().find(|e| e.id == digest.id) {
            Some(existing_entry) => existing_entry.digest = digest,
            None => {
                return Err(Status::not_found(format!(
                    "update_entry() called for game_id={game_id} but entry was not found in library."
                )));
            }
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

#[instrument(name = "wishlist::read", level = "trace", skip(firestore, user_id))]
async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<Library, Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(GAMES)
        .parent(&parent_path)
        .obj()
        .one(WISHLIST_DOC)
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Ok(Library { entries: vec![] }),
    }
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

    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    firestore
        .db()
        .fluent()
        .update()
        .in_col(GAMES)
        .document_id(WISHLIST_DOC)
        .parent(&parent_path)
        .object(&library)
        .execute()
        .await?;
    Ok(())
}

const USERS: &str = "users";
const GAMES: &str = "games";
const WISHLIST_DOC: &str = "wishlist";
