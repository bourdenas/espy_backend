use crate::{api::FirestoreApi, documents::Collection, Status};
use tracing::instrument;

#[instrument(name = "franchises::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Collection, Status> {
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(FRANCHISES)
        .obj()
        .one(doc_id.to_string())
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{FRANCHISES}/{doc_id}' was not found"
        ))),
    }
}

#[instrument(
    name = "franchises::write",
    level = "trace",
    skip(firestore, franchise)
    fields(
        franchise = %franchise.slug,
    )
)]
pub async fn write(firestore: &FirestoreApi, franchise: &Collection) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(FRANCHISES)
        .document_id(franchise.id.to_string())
        .object(franchise)
        .execute()
        .await?;
    Ok(())
}

const FRANCHISES: &str = "franchises";
