use crate::{api::FirestoreApi, documents::IgdbCollection, Status};
use tracing::instrument;

/// Returns a list of all franchises docs stored on Firestore.
#[instrument(name = "franchises::list", level = "trace", skip(firestore))]
pub fn list(firestore: &FirestoreApi) -> Result<Vec<IgdbCollection>, Status> {
    firestore.list(&format!("franchises"))
}

/// Returns a franchise doc based on its `id` from Firestore.
#[instrument(name = "franchises::read", level = "trace", skip(firestore))]
pub fn read(firestore: &FirestoreApi, id: u64) -> Result<IgdbCollection, Status> {
    firestore.read::<IgdbCollection>("franchises", &id.to_string())
}

/// Writes franchise doc in Firestore.
#[instrument(
    name = "franchises::write",
    level = "trace",
    skip(firestore, franchice)
    fields(
        franchice = %franchice.slug,
    )
)]
pub fn write(firestore: &FirestoreApi, franchice: &IgdbCollection) -> Result<(), Status> {
    firestore.write("franchises", Some(&franchice.id.to_string()), franchice)?;
    Ok(())
}

/// Deletes a franchise doc from Firestore.
#[instrument(name = "franchises::delete", level = "trace", skip(firestore))]
pub fn delete(firestore: &FirestoreApi, id: u64) -> Result<(), Status> {
    firestore.delete(&format!("franchises/{id}"))
}
