use firestore::FirestoreDb;
use tracing::instrument;

use crate::{
    documents::{NotableCompanies, Timeline},
    Status,
};

#[instrument(name = "timeline::write", level = "trace", skip(db))]
pub async fn write(db: &FirestoreDb, timeline: &Timeline) -> Result<(), Status> {
    db.fluent()
        .update()
        .in_col("espy")
        .document_id("timeline")
        .object(timeline)
        .execute()
        .await?;
    Ok(())
}

#[instrument(name = "timeline::read_notable", level = "trace", skip(db))]
pub async fn read_notable(db: &FirestoreDb) -> Result<NotableCompanies, Status> {
    Ok(db
        .fluent()
        .select()
        .by_id_in("espy")
        .obj()
        .one("notable")
        .await?
        .unwrap_or_default())
}
