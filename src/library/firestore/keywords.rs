use tracing::instrument;

use crate::{api::FirestoreApi, documents::Keyword, Status};

/// Returns a list of all Keywords docs stored on Firestore.
#[instrument(name = "keywords::list", level = "trace", skip(firestore))]
pub fn list(firestore: &FirestoreApi) -> Result<Vec<Keyword>, Status> {
    firestore.list(&format!("keywords"))
}

/// Returns an Keyword doc based on its `id` from Firestore.
#[instrument(name = "keywords::read", level = "trace", skip(firestore))]
pub fn read(firestore: &FirestoreApi, id: u64) -> Result<Keyword, Status> {
    firestore.read::<Keyword>("keywords", &id.to_string())
}

/// Writes an Keyword doc in Firestore.
#[instrument(name = "keywords::write", level = "trace", skip(firestore))]
pub fn write(firestore: &FirestoreApi, keyword: &Keyword) -> Result<(), Status> {
    firestore.write("keywords", Some(&keyword.id.to_string()), keyword)?;
    Ok(())
}

/// Deletes an Keyword doc from Firestore.
#[instrument(name = "keywords::delete", level = "trace", skip(firestore))]
pub fn delete(firestore: &FirestoreApi, id: u64) -> Result<(), Status> {
    firestore.delete(&format!("keywords/{id}"))
}
