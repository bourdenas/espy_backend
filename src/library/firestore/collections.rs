use crate::{api::FirestoreApi, documents::Collection, Status};
use futures::{stream::BoxStream, StreamExt};
use tracing::instrument;

#[instrument(name = "collections::list", level = "trace", skip(firestore))]
pub async fn list(firestore: &FirestoreApi) -> Result<Vec<Collection>, Status> {
    let doc_stream: BoxStream<Collection> = firestore
        .db()
        .fluent()
        .list()
        .from(COLLECTIONS)
        .obj()
        .stream_all()
        .await?;

    Ok(doc_stream.collect().await)
}

#[instrument(name = "collections::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Collection, Status> {
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(COLLECTIONS)
        .obj()
        .one(doc_id.to_string())
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{COLLECTIONS}/{doc_id}' was not found"
        ))),
    }
}

#[instrument(
    name = "collections::write",
    level = "trace",
    skip(firestore, collection)
    fields(
        collection = %collection.slug,
    )
)]
pub async fn write(firestore: &FirestoreApi, collection: &Collection) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(COLLECTIONS)
        .document_id(collection.id.to_string())
        .object(collection)
        .execute()
        .await?
}

#[instrument(name = "collections::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, doc_id: u64) -> Result<(), Status> {
    Ok(firestore
        .db()
        .fluent()
        .delete()
        .from(COLLECTIONS)
        .document_id(doc_id.to_string())
        .execute()
        .await?)
}

const COLLECTIONS: &str = "collections";
