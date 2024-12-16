use tracing::instrument;

use crate::{api::FirestoreApi, documents::Frontpage, Status};

use super::utils;

#[instrument(name = "frontpage::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, frontpage: &Frontpage) -> Result<(), Status> {
    utils::write(firestore, "espy", "frontpage".to_string(), frontpage).await
}
