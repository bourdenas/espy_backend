use crate::{api::FirestoreApi, documents::IgdbCollection, Status};
use tracing::instrument;

/// Returns a list of all Collection docs stored on Firestore.
#[instrument(name = "collections::list", level = "trace", skip(firestore))]
pub fn list(firestore: &FirestoreApi) -> Result<Vec<IgdbCollection>, Status> {
    firestore.list(&format!("collections"))
}

/// Returns an IgdbCollection doc based on its `slug` from Firestore.
#[instrument(name = "collections::read", level = "trace", skip(firestore))]
pub fn read(firestore: &FirestoreApi, slug: &str) -> Result<IgdbCollection, Status> {
    firestore.read::<IgdbCollection>("collections", slug)
}

/// Writes an IgdbCollection doc in Firestore.
#[instrument(
    name = "collections::write",
    level = "trace",
    skip(firestore, collection)
    fields(
        collection = %collection.slug,
    )
)]
pub fn write(firestore: &FirestoreApi, collection: &IgdbCollection) -> Result<(), Status> {
    firestore.write("collections", Some(&collection.slug), collection)?;
    Ok(())
}

/// Deletes an IgdbCollection doc from Firestore.
#[instrument(name = "collections::delete", level = "trace", skip(firestore))]
pub fn delete(firestore: &FirestoreApi, slug: &str) -> Result<(), Status> {
    firestore.delete(&format!("collections/{}", slug))
}
