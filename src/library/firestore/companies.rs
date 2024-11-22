use firestore::path;
use futures::{stream::BoxStream, StreamExt};
use tracing::{debug, instrument};

use crate::{
    api::FirestoreApi,
    documents::Company,
    log,
    logging::{Criterion, FirestoreEvent},
    Status,
};

use super::{utils, BatchReadResult};

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
    utils::read(firestore, COMPANIES, doc_id.to_string()).await
}

#[instrument(
    name = "companies::batch_read",
    level = "trace",
    skip(firestore, doc_ids)
)]
pub async fn batch_read(
    firestore: &FirestoreApi,
    doc_ids: &[u64],
) -> Result<BatchReadResult<Company>, Status> {
    utils::batch_read(firestore, COMPANIES, doc_ids).await
}

#[instrument(name = "companies::search", level = "trace", skip(firestore))]
pub async fn search(firestore: &FirestoreApi, slug: &str) -> Result<Vec<Company>, Status> {
    let result = firestore
        .db()
        .fluent()
        .select()
        .from(COMPANIES)
        .filter(|q| q.for_all([q.field(path!(Company::slug)).equal(slug)]))
        .obj()
        .stream_query_with_errors()
        .await;

    match result {
        Ok(mut stream) => {
            let mut companies = vec![];
            let mut errors = vec![];
            while let Some(company) = stream.next().await {
                match company {
                    Ok(company) => companies.push(company),
                    Err(e) => errors.push(e.to_string()),
                }
            }

            log!(FirestoreEvent::search(
                format!("/{COMPANIES}"),
                vec![Criterion::new("slug".to_owned(), slug.to_string())],
                companies.len(),
                errors.len(),
                errors,
            ));
            Ok(companies)
        }
        Err(e) => {
            log!(FirestoreEvent::search(
                format!("/{COMPANIES}"),
                vec![Criterion::new("slug".to_owned(), slug.to_string())],
                0,
                1,
                vec![e.to_string()],
            ));
            Err(utils::make_status(e, COMPANIES, format!("slug = {slug}")))
        }
    }
}

#[instrument(name = "companies::write", level = "trace", skip(firestore, company))]
pub async fn write(firestore: &FirestoreApi, company: &Company) -> Result<(), Status> {
    utils::write(firestore, COMPANIES, company.id.to_string(), company).await
}

#[instrument(name = "companies::delete", level = "trace", skip(firestore))]
pub async fn delete(firestore: &FirestoreApi, doc_id: u64) -> Result<(), Status> {
    utils::delete(firestore, COMPANIES, doc_id.to_string()).await
}

const COMPANIES: &str = "companies";
