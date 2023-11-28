use tracing::instrument;

use crate::{
    api::FirestoreApi,
    documents::{NotableCompanies, Timeline},
    Status,
};

#[instrument(name = "timeline::write", level = "trace", skip(firestore))]
pub async fn write(firestore: &FirestoreApi, timeline: &Timeline) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col("espy")
        .document_id("timeline")
        .object(timeline)
        .execute()
        .await?;
    Ok(())
}

#[instrument(name = "timeline::read_notable", level = "trace", skip(firestore))]
pub async fn read_notable(firestore: &FirestoreApi) -> Result<NotableCompanies, Status> {
    Ok(firestore
        .db()
        .fluent()
        .select()
        .by_id_in("espy")
        .obj()
        .one("notable")
        .await?
        .unwrap_or_default())
}
