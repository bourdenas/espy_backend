use tracing::instrument;

use crate::{api::FirestoreApi, documents::Notable, Status};

use super::utils;

#[instrument(name = "notable::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi) -> Result<Notable, Status> {
    Ok(utils::read(firestore, "espy", "notable".to_string())
        .await
        .unwrap_or_default())
}

#[instrument(name = "notable::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, notable: &Notable) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col("espy")
        .document_id("notable")
        .object(notable)
        .execute::<()>()
        .await?;
    Ok(())
}
