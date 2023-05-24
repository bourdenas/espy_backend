use tracing::instrument;

use crate::{api::FirestoreApi, documents::Genre, Status};

/// Returns a list of all Genres docs stored on Firestore.
#[instrument(name = "genres::list", level = "trace", skip(firestore))]
pub fn list(firestore: &FirestoreApi) -> Result<Vec<Genre>, Status> {
    firestore.list(&format!("genres"))
}

/// Returns an Genre doc based on its `id` from Firestore.
#[instrument(name = "genres::read", level = "trace", skip(firestore))]
pub fn read(firestore: &FirestoreApi, id: u64) -> Result<Genre, Status> {
    firestore.read::<Genre>("genres", &id.to_string())
}

/// Writes an Genre doc in Firestore.
#[instrument(name = "genres::write", level = "trace", skip(firestore))]
pub fn write(firestore: &FirestoreApi, genre: &Genre) -> Result<(), Status> {
    firestore.write("genres", Some(&genre.id.to_string()), genre)?;
    Ok(())
}

/// Deletes an Genre doc from Firestore.
#[instrument(name = "genres::delete", level = "trace", skip(firestore))]
pub fn delete(firestore: &FirestoreApi, id: u64) -> Result<(), Status> {
    firestore.delete(&format!("genres/{id}"))
}
