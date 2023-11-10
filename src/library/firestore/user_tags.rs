use crate::{api::FirestoreApi, documents::UserTags, Status};
use tracing::instrument;

#[instrument(
    name = "user_tags::add_user_tag",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn add_user_tag(
    firestore: &FirestoreApi,
    user_id: &str,
    game_id: u64,
    tag_name: String,
    class_name: Option<&str>,
) -> Result<(), Status> {
    let mut user_tags = read(firestore, user_id).await?;
    if user_tags.add(game_id, tag_name, class_name) {
        write(firestore, user_id, &user_tags).await?;
    }
    Ok(())
}

#[instrument(
    name = "user_tags::remove_user_tag",
    level = "trace",
    skip(firestore, user_id)
)]
pub async fn remove_user_tag(
    firestore: &FirestoreApi,
    user_id: &str,
    game_id: u64,
    tag_name: &str,
    class_name: Option<&str>,
) -> Result<(), Status> {
    let mut user_tags = read(firestore, user_id).await?;
    if user_tags.remove(game_id, tag_name, class_name) {
        write(firestore, user_id, &user_tags).await?;
    }
    Ok(())
}

#[instrument(name = "user_tags::read", level = "trace", skip(firestore, user_id))]
async fn read(firestore: &FirestoreApi, user_id: &str) -> Result<UserTags, Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(USER_DATA)
        .parent(&parent_path)
        .obj()
        .one(TAGS_DOC)
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{USERS}/{user_id}/{USER_DATA}/{TAGS_DOC}' was not found"
        ))),
    }
}

#[instrument(
    name = "user_tags::write",
    level = "trace",
    skip(firestore, user_id, user_tags)
)]
async fn write(
    firestore: &FirestoreApi,
    user_id: &str,
    user_tags: &UserTags,
) -> Result<(), Status> {
    let parent_path = firestore.db().parent_path(USERS, user_id)?;

    firestore
        .db()
        .fluent()
        .update()
        .in_col(USER_DATA)
        .document_id(TAGS_DOC)
        .parent(&parent_path)
        .object(user_tags)
        .execute()
        .await?;
    Ok(())
}

const USERS: &str = "users";
const USER_DATA: &str = "user_data";
const TAGS_DOC: &str = "tags";
