use tracing::instrument;

use crate::{api::FirestoreApi, documents::AnnualReview, Status};

use super::utils;

#[instrument(name = "year::write", level = "trace", skip(firestore))]
pub async fn write(
    firestore: &FirestoreApi,
    review: &AnnualReview,
    year: u64,
) -> Result<(), Status> {
    utils::write(firestore, "espy", format!("{year}"), review).await
}
