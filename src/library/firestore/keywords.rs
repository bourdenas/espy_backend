use tracing::instrument;

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
        .await?
}

const KEYWORDS: &str = "keywords";
