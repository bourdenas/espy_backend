use crate::{api::FirestoreApi, documents::Collection, Status};
use firestore::FirestoreResult;
use futures::{stream::BoxStream, StreamExt};
use tracing::{instrument, warn};

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

/// Batch reads franchises by id.
///
/// Returns a tuple with two vectors. The first one contains the found Franchice
/// docs and the second contains the doc ids that were not found.
#[instrument(
    name = "franchices::batch_read",
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
        .by_id_in(FRANCHISES)
        .obj()
        .batch_with_errors(doc_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>())
        .await?;

    let mut franchises = vec![];
    let mut not_found = vec![];
    while let Some(franchise) = docs.next().await {
        match franchise {
            Ok((id, franchise)) => match franchise {
                Some(franchise) => franchises.push(franchise),
                None => not_found.push(id.parse().unwrap_or_default()),
            },
            Err(status) => warn!("{status}"),
        }
    }

    Ok((franchises, not_found))
}

const FRANCHISES: &str = "franchises";
