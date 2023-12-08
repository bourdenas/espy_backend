use firestore::FirestoreResult;
use futures::{stream::BoxStream, StreamExt};
use tracing::{instrument, warn};

use crate::{api::FirestoreApi, documents::Keyword, Status};

#[instrument(name = "keywords::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Keyword, Status> {
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(KEYWORDS)
        .obj()
        .one(doc_id.to_string())
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{KEYWORDS}/{doc_id}' was not found"
        ))),
    }
}

#[instrument(name = "keywords::batch_read", level = "trace", skip(firestore))]
pub async fn batch_read(firestore: &FirestoreApi, doc_ids: &[u64]) -> Result<Vec<Keyword>, Status> {
    let mut docs: BoxStream<FirestoreResult<(String, Option<Keyword>)>> = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(KEYWORDS)
        .obj()
        .batch_with_errors(doc_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>())
        .await?;

    let mut keywords: Vec<Keyword> = vec![];
    while let Some(kw) = docs.next().await {
        match kw {
            Ok((_, kw)) => match kw {
                Some(kw) => keywords.push(kw),
                None => {}
            },
            Err(status) => warn!("{status}"),
        }
    }

    Ok(keywords)
}

#[instrument(name = "keywords::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, keyword: &Keyword) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(KEYWORDS)
        .document_id(keyword.id.to_string())
        .object(keyword)
        .execute()
        .await?;
    Ok(())
}

const KEYWORDS: &str = "keywords";
