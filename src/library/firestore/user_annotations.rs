use crate::{api::FirestoreApi, documents::UserAnnotations, Status};
use tracing::instrument;

use super::utils;

#[instrument(
    name = "user_annotations::read",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<UserAnnotations, Status> {
    utils::auth_read(firestore, user_id, USER_DATA, TAGS_DOC.to_owned()).await
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
    utils::auth_write(
        firestore,
        user_id,
        USER_DATA,
        TAGS_DOC.to_owned(),
        user_annotations,
    )
    .await
}

const USER_DATA: &str = "user_data";
const TAGS_DOC: &str = "tags";
