use tracing::instrument;

use crate::{api::FirestoreApi, documents::AnnualReview, Status};

#[instrument(name = "year::write", level = "trace", skip(firestore))]
pub async fn write(
    firestore: &FirestoreApi,
    review: &AnnualReview,
    year: u64,
) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col("espy")
        .document_id(format!("{year}"))
        .object(review)
        .execute::<()>()
        .await?;
    Ok(())
}
