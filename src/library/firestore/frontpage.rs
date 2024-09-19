use tracing::instrument;

use crate::{api::FirestoreApi, documents::Frontpage, Status};

#[instrument(name = "frontpage::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, frontpage: &Frontpage) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col("espy")
        .document_id("frontpage")
        .object(frontpage)
        .execute::<()>()
        .await?;
    Ok(())
}
