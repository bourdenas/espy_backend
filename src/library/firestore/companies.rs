use crate::{api::FirestoreApi, documents::Company, Status};
use futures::{stream::BoxStream, StreamExt};
use tracing::instrument;

#[instrument(name = "companies::list", level = "trace", skip(firestore))]
pub async fn list(firestore: &FirestoreApi) -> Result<Vec<Company>, Status> {
    let doc_stream: BoxStream<Company> = firestore
        .db()
        .fluent()
        .list()
        .from(COMPANIES)
        .obj()
        .stream_all()
        .await?;

    Ok(doc_stream.collect().await)
}

#[instrument(name = "companies::read", level = "trace", skip(firestore))]
pub async fn read(firestore: &FirestoreApi, doc_id: u64) -> Result<Company, Status> {
    let doc = firestore
        .db()
        .fluent()
        .select()
        .by_id_in(COMPANIES)
        .obj()
        .one(doc_id.to_string())
        .await?;

    match doc {
        Some(doc) => Ok(doc),
        None => Err(Status::not_found(format!(
            "Firestore document '{COMPANIES}/{doc_id}' was not found"
        ))),
    }
}

#[instrument(
    name = "companies::write",
    level = "trace",
    skip(firestore, company)
    fields(
        company = %company.slug,
    )
)]
pub async fn write(firestore: &FirestoreApi, company: &Company) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .update()
        .in_col(COMPANIES)
        .document_id(company.id.to_string())
        .object(company)
        .execute()
        .await?;
    Ok(())
}

#[instrument(name = "companies::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, doc_id: u64) -> Result<(), Status> {
    firestore
        .db()
        .fluent()
        .delete()
        .from(COMPANIES)
        .document_id(doc_id.to_string())
        .execute()
        .await?;
    Ok(())
}

const COMPANIES: &str = "companies";
