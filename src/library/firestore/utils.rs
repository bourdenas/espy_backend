use std::fmt::Display;

use firestore::{errors::FirestoreError, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use tracing::info;

use crate::{api::FirestoreApi, logging::FirestoreEvent, Status};

pub async fn read<Document: serde::de::DeserializeOwned + Send>(
    firestore: &FirestoreApi,
    collection: &str,
    doc_id: String,
) -> Result<Document, Status> {
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(collection)
        .obj()
        .one(doc_id.clone())
        .await;

    match doc {
        Ok(doc) => match doc {
            Some(doc) => {
                info!(event = FirestoreEvent::read(collection.to_owned(), doc_id, None).encode());
                Ok(doc)
            }
            None => {
                let status = Status::not_found(format!(
                    "Firestore '{collection}/{doc_id}' document was not found"
                ));
                info!(
                    event = FirestoreEvent::read_not_found(collection.to_owned(), doc_id, None)
                        .encode()
                );
                Err(status)
            }
        },
        Err(e) => {
            info!(
                event = FirestoreEvent::read(
                    collection.to_owned(),
                    doc_id.to_owned(),
                    Some(e.to_string()),
                )
                .encode()
            );
            Err(make_status(e, collection, doc_id))
        }
    }
}

// Reads from the /users/{id} collection. Returns a default doc if one is not
// found.
pub async fn users_read<Document: serde::de::DeserializeOwned + Default + Send>(
    firestore: &FirestoreApi,
    user_id: &str,
    collection: &str,
    doc_id: &str,
) -> Result<Document, Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(collection)
        .parent(&parent_path)
        .obj()
        .one(doc_id)
        .await;

    let collection = format!("{USERS}/{user_id}/{collection}");
    match doc {
        Ok(doc) => match doc {
            Some(doc) => {
                info!(
                    event = FirestoreEvent::read(collection.to_owned(), doc_id.to_owned(), None)
                        .encode()
                );
                Ok(doc)
            }
            None => {
                info!(
                    event = FirestoreEvent::read_not_found(
                        collection.to_owned(),
                        doc_id.to_owned(),
                        None,
                    )
                    .encode()
                );
                Ok(Document::default())
            }
        },
        Err(e) => {
            info!(
                event = FirestoreEvent::read(
                    collection.to_owned(),
                    doc_id.to_owned(),
                    Some(e.to_string()),
                )
                .encode()
            );
            Err(make_status(e, &collection, doc_id))
        }
    }
}

pub async fn batch_read<Document: serde::de::DeserializeOwned + Send>(
    firestore: &FirestoreApi,
    collection: &str,
    doc_ids: &[u64],
) -> Result<BatchReadResult<Document>, Status> {
    let mut docs: BoxStream<FirestoreResult<(String, Option<Document>)>> = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(collection)
        .obj()
        .batch_with_errors(doc_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>())
        .await?;

    let mut documents = vec![];
    let mut not_found = vec![];
    let mut errors = vec![];
    while let Some(doc) = docs.next().await {
        match doc {
            Ok((id, doc)) => match doc {
                Some(doc) => documents.push(doc),
                None => not_found.push(id.parse().unwrap_or_default()),
            },
            // The API does not return the doc_id that caused the error.
            Err(e) => errors.push(e.to_string()),
        }
    }

    info!(
        event = FirestoreEvent::batch(
            collection.to_owned(),
            documents.len(),
            not_found.len(),
            errors,
        )
        .encode()
    );

    Ok(BatchReadResult {
        documents,
        not_found,
    })
}

pub async fn write<Document: serde::Serialize + serde::de::DeserializeOwned + Send + Sync>(
    firestore: &FirestoreApi,
    collection: &str,
    doc_id: &str,
    document: &Document,
) -> Result<(), Status> {
    let result = firestore
        .db()
        .fluent()
        .update()
        .in_col(collection)
        .document_id(doc_id)
        .object(document)
        .execute::<()>()
        .await;

    match result {
        Ok(()) => {
            info!(
                event =
                    FirestoreEvent::write(collection.to_owned(), doc_id.to_owned(), None).encode()
            );
            Ok(())
        }
        Err(e) => {
            info!(
                event = FirestoreEvent::write(
                    collection.to_owned(),
                    doc_id.to_owned(),
                    Some(e.to_string()),
                )
                .encode()
            );
            Err(make_status(e, collection, doc_id))
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchReadResult<Document> {
    pub documents: Vec<Document>,
    pub not_found: Vec<u64>,
}

pub fn make_status<S: Into<String> + Display>(
    error: FirestoreError,
    collection: &str,
    doc_id: S,
) -> Status {
    match error {
        FirestoreError::DeserializeError(e) => Status::internal(format!(
            "Firestore '{collection}/{doc_id}' document failed to parse with error '{}'",
            e.message,
        )),
        e => Status::internal(format!("Firestore '{collection}/{doc_id}' error: {e}")),
    }
}

pub const USERS: &str = "users";
