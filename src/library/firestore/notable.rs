use tracing::instrument;

use crate::{api::FirestoreApi, documents::Notable, Status};

#[instrument(name = "notable::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi) -> Result<Notable, Status> {
    Ok(firestore
        .db()
        .fluent()
        .select()
        .by_id_in("espy")
        .obj()
        .one("notable")
        .await?
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
        .execute()
        .await?;
    Ok(())
}
