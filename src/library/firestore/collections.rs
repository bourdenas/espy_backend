use crate::{api::FirestoreApi, documents::Collection, Status};
use firestore::FirestoreResult;
use futures::{stream::BoxStream, StreamExt};
use tracing::{instrument, warn};

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
        .await?;
    Ok(())
}

/// Batch reads collections by id.
///
/// Returns a tuple with two vectors. The first one contains the found
/// Collection docs and the second contains the doc ids that were not found.
#[instrument(
    name = "collections::batch_read",
    level = "trace",
    skip(firestore, doc_ids)
)]
pub async fn batch_read(
    firestore: &FirestoreApi,
    doc_ids: &[u64],
) -> Result<(Vec<Collection>, Vec<u64>), Status> {
    let mut docs: BoxStream<FirestoreResult<(String, Option<Collection>)>> = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(COLLECTIONS)
        .obj()
        .batch_with_errors(doc_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>())
        .await?;

    let mut collections = vec![];
    let mut not_found = vec![];
    while let Some(collection) = docs.next().await {
        match collection {
            Ok((id, collection)) => match collection {
                Some(collection) => collections.push(collection),
                None => not_found.push(id.parse().unwrap_or_default()),
            },
            Err(status) => warn!("{status}"),
        }
    }

    Ok((collections, not_found))
}

#[instrument(name = "collections::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, doc_id: u64) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .delete()
        .from(COLLECTIONS)
        .document_id(doc_id.to_string())
        .execute()
        .await?;
    Ok(())
}

const COLLECTIONS: &str = "collections";
