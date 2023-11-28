use tracing::instrument;

use crate::{api::FirestoreApi, documents::UserData, Status};

#[instrument(name = "users::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: &str) -> Result<UserData, Status> {
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(USERS)
        .obj()
        .one(doc_id)
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{USERS}/{doc_id}' was not found"
        ))),
    }
}

#[instrument(name = "users::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, user_data: &UserData) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(USERS)
        .document_id(&user_data.uid)
        .object(user_data)
        .execute()
        .await?;
    Ok(())
}

const USERS: &str = "users";
