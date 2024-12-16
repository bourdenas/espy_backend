use tracing::instrument;

use crate::{api::FirestoreApi, documents::UserData, Status};

use super::utils;

#[instrument(name = "users::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: &str) -> Result<UserData, Status> {
    utils::read(firestore, USERS, doc_id.to_owned()).await
}

#[instrument(name = "users::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, user_data: &UserData) -> Result<(), Status> {
    utils::write(firestore, USERS, user_data.uid.clone(), user_data).await
}

const USERS: &str = "users";
