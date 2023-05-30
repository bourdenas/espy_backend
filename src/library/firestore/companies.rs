use crate::{api::FirestoreApi, documents::Company, Status};
use tracing::instrument;

/// Returns a list of all Collection docs stored on Firestore.
#[instrument(name = "companies::list", level = "trace", skip(firestore))]
pub fn list(firestore: &FirestoreApi) -> Result<Vec<Company>, Status> {
    firestore.list(&format!("companies"))
}

/// Returns an IgdbCompany doc based on its `slug` from Firestore.
#[instrument(name = "companies::read", level = "trace", skip(firestore))]
pub fn read(firestore: &FirestoreApi, slug: &str) -> Result<Company, Status> {
    firestore.read::<Company>("companies", slug)
}

/// Returns an IgdbCompany doc based on its `slug` from Firestore.
#[instrument(name = "companies::read", level = "trace", skip(firestore))]
pub fn search(firestore: &FirestoreApi, id: u64) -> Result<Vec<Company>, Status> {
    firestore.query::<Company>("companies", "id", id.into())
}

/// Writes an IgdbCompany doc in Firestore.
#[instrument(
    name = "companies::write",
    level = "trace",
    skip(firestore, collection)
    fields(
        collection = %collection.slug,
    )
)]
pub fn write(firestore: &FirestoreApi, collection: &Company) -> Result<(), Status> {
    firestore.write("companies", Some(&collection.slug), collection)?;
    Ok(())
}

/// Deletes an IgdbCompany doc from Firestore.
#[instrument(name = "companies::delete", level = "trace", skip(firestore))]
pub fn delete(firestore: &FirestoreApi, slug: &str) -> Result<(), Status> {
    firestore.delete(&format!("companies/{}", slug))
}
