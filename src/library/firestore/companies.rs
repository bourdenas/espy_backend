use crate::{api::FirestoreApi, documents::Company, Status};
use tracing::instrument;

/// Returns a list of all Company docs stored on Firestore.
#[instrument(name = "companies::list", level = "trace", skip(firestore))]
pub fn list(firestore: &FirestoreApi) -> Result<Vec<Company>, Status> {
    firestore.list(&format!("companies"))
}

/// Returns an IgdbCompany doc based on its `id` from Firestore.
#[instrument(name = "companies::read", level = "trace", skip(firestore))]
pub fn read(firestore: &FirestoreApi, id: u64) -> Result<Company, Status> {
    firestore.read::<Company>("companies", &id.to_string())
}

/// Writes an IgdbCompany doc in Firestore.
#[instrument(
    name = "companies::write",
    level = "trace",
    skip(firestore, company)
    fields(
        company_id = %company.id,
        company = %company.slug,
    )
)]
pub fn write(firestore: &FirestoreApi, company: &Company) -> Result<(), Status> {
    firestore.write("companies", Some(&company.id.to_string()), company)?;
    Ok(())
}

/// Deletes an IgdbCompany doc from Firestore.
#[instrument(name = "companies::delete", level = "trace", skip(firestore))]
pub fn delete(firestore: &FirestoreApi, id: u64) -> Result<(), Status> {
    firestore.delete(&format!("companies/{}", &id.to_string()))
}
