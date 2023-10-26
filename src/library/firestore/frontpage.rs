use tracing::instrument;

use crate::{api::FirestoreApi, documents::Frontpage, Status};

#[instrument(name = "genres::read", level = "trace", skip(firestore))]
pub fn read(firestore: &FirestoreApi) -> Result<Frontpage, Status> {
    firestore.read::<Frontpage>("espy", "frontpage")
}

#[instrument(name = "genres::write", level = "trace", skip(firestore))]
pub fn write(firestore: &FirestoreApi, frontpage: &Frontpage) -> Result<(), Status> {
    firestore.write("espy", Some("frontpage"), frontpage)?;
    Ok(())
}
