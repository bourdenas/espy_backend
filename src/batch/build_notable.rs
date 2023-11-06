use clap::Parser;
use espy_backend::{
    documents::{Library, NotableCompanies},
    Status, Tracing,
};
use firestore::FirestoreDb;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Parser)]
struct Opts {
    #[clap(long)]
    prod_tracing: bool,

    #[clap(long)]
    user: String,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    let opts: Opts = Opts::parse();

    match opts.prod_tracing {
        false => Tracing::setup("build-notable")?,
        true => Tracing::setup_prod("build-notable")?,
    }

    let user = &opts.user;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let db = FirestoreDb::new("espy-library").await?;

    let library: Option<Library> = db
        .fluent()
        .select()
        .by_id_in(&format!("users/{user}/games"))
        .obj()
        .one("library")
        .await?;

    let mut companies = HashSet::<String>::new();
    for library_entry in library.unwrap().entries {
        for company in library_entry
            .digest
            .developers
            .into_iter()
            .chain(library_entry.digest.publishers.into_iter())
        {
            companies.insert(company);
        }
    }

    let notable = NotableCompanies {
        companies: Vec::from_iter(companies.into_iter()),
        last_updated: now,
    };

    db.fluent()
        .update()
        .in_col("espy")
        .document_id("notable")
        .object(&notable)
        .execute()
        .await?;

    Ok(())
}
