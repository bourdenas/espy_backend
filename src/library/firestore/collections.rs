use crate::{api::FirestoreApi, documents::Collection, Status};
use tracing::instrument;

/// Returns a list of all collection docs stored on Firestore.
#[instrument(name = "collections::list", level = "trace", skip(firestore))]
pub fn list(firestore: &FirestoreApi) -> Result<Vec<Collection>, Status> {
    firestore.list(&format!("collections"))
}

/// Returns a collection doc based on its `id` from Firestore.
#[instrument(name = "collections::read", level = "trace", skip(firestore))]
pub fn read(firestore: &FirestoreApi, id: u64) -> Result<Collection, Status> {
    firestore.read::<Collection>("collections", &id.to_string())
}

/// Writes a collection doc in Firestore.
#[instrument(
    name = "collections::write",
    level = "trace",
    skip(firestore, collection)
    fields(
        collection = %collection.slug,
    )
)]
pub fn write(firestore: &FirestoreApi, collection: &Collection) -> Result<(), Status> {
    firestore.write("collections", Some(&collection.id.to_string()), collection)?;
    Ok(())
}

/// Deletes a collection doc from Firestore.
#[instrument(name = "collections::delete", level = "trace", skip(firestore))]
pub fn delete(firestore: &FirestoreApi, id: u64) -> Result<(), Status> {
    firestore.delete(&format!("collections/{id}"))
}
