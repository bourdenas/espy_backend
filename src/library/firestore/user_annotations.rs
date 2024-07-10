use crate::{api::FirestoreApi, documents::UserAnnotations, Status};
use tracing::instrument;

use super::utils;

#[instrument(
    name = "user_annotations::read",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<UserAnnotations, Status> {
    utils::users_read(firestore, user_id, USER_DATA, TAGS_DOC).await
}

#[instrument(
    name = "user_annotations::write",
    level = "trace",
    skip(firestore, user_id, user_annotations)
)]
async fn write(
    firestore: &FirestoreApi,
    user_id: &str,
    user_annotations: &UserAnnotations,
) -> Result<(), Status> {
    let parent_path = firestore.db().parent_path(utils::USERS, user_id)?;

    firestore
        .db()
        .fluent()
        .update()
        .in_col(USER_DATA)
        .document_id(TAGS_DOC)
        .parent(&parent_path)
        .object(user_annotations)
        .execute()
        .await?;
    Ok(())
}

const USER_DATA: &str = "user_data";
const TAGS_DOC: &str = "tags";
