use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use espy_backend::{
    api::{CompanyNormalizer, FirestoreApi},
    documents::{Company, GameDigest},
    library,
    resolver::{IgdbConnection, IgdbLookup},
    util, Tracing,
};
use firestore::{struct_path::path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use itertools::Itertools;
use tracing::{error, warn};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "0")]
    cursor: u64,

    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_companies")?;

    let opts: Opts = Opts::parse();
    let mut cursor = opts.cursor;

    let keys = util::keys::Keys::from_file(&opts.key_store).unwrap();
    let connection = IgdbConnection::new(&keys.igdb.client_id, &keys.igdb.secret).await?;
    let lookup = IgdbLookup::new(&connection);

    let mut i = 0;
    while i % BATCH_SIZE == 0 {
        let firestore = Arc::new(FirestoreApi::connect().await?);

        let mut companies: BoxStream<FirestoreResult<Company>> = firestore
            .db()
            .fluent()
            .select()
            .from("companies")
            .filter(|q| q.for_all([q.field(path!(Company::id)).greater_than_or_equal(cursor)]))
            .order_by([(path!(Company::id), FirestoreQueryDirection::Ascending)])
            .limit(BATCH_SIZE)
            .obj()
            .stream_query_with_errors()
            .await?;

        let mut tbd = vec![];
        while let Some(company) = companies.next().await {
            match company {
                Ok(mut company) => {
                    cursor = company.id;

                    let start = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();

                    let igdb_company = lookup
                        .get_companies(&vec![company.id])
                        .await?
                        .first()
                        .unwrap()
                        .clone();
                    company.description = igdb_company.description.clone();
                    if let Some(logo) = igdb_company.logo {
                        company.logo = lookup.get_company_logo(logo).await?;
                    }
                    company.slug = CompanyNormalizer::slug(&company.name);

                    company
                        .developed
                        .retain(|digest| digest.category.is_main_category());
                    company.developed.sort_by(|l, r| l.id.cmp(&r.id));
                    company.developed.dedup_by_key(|e| e.id);

                    company
                        .published
                        .retain(|digest| digest.category.is_main_category());
                    company.published.sort_by(|l, r| l.id.cmp(&r.id));
                    company.published.dedup_by_key(|e| e.id);

                    println!(
                        "#{i} -- {} -- id={} -- developed {} games -- published {} games)",
                        company.name,
                        company.id,
                        company.developed.len(),
                        company.published.len(),
                    );

                    if company.developed.len() + company.published.len() == 0 {
                        tbd.push(company.clone());
                    }

                    let mut developed_games = vec![];
                    if !company.developed.is_empty() {
                        let result = library::firestore::games::batch_read(
                            &firestore,
                            &company.developed.iter().map(|e| e.id).collect_vec(),
                        )
                        .await?;

                        if !result.not_found.is_empty() {
                            warn!(
                                "missing {} developed GameEntries from company '{}' ({})",
                                result.not_found.len(),
                                &company.name,
                                company.id,
                            );
                        }
                        developed_games = result
                            .documents
                            .into_iter()
                            .filter(|game_entry| {
                                game_entry
                                    .developers
                                    .iter()
                                    .find(|digest| digest.name == company.name)
                                    .is_some()
                            })
                            .collect();
                    }

                    let mut published_games = vec![];
                    if !company.published.is_empty() {
                        let result = library::firestore::games::batch_read(
                            &firestore,
                            &company.published.iter().map(|e| e.id).collect_vec(),
                        )
                        .await?;

                        if !result.not_found.is_empty() {
                            warn!(
                                "missing {} published GameEntries from company '{}' ({})",
                                result.not_found.len(),
                                &company.name,
                                company.id,
                            );
                        }
                        published_games = result
                            .documents
                            .into_iter()
                            .filter(|game_entry| {
                                game_entry
                                    .publishers
                                    .iter()
                                    .find(|digest| digest.name == company.name)
                                    .is_some()
                            })
                            .collect();
                    }

                    let company = Company {
                        id: company.id,
                        slug: CompanyNormalizer::slug(&company.name),
                        name: company.name,
                        description: company.description,
                        logo: company.logo,
                        developed: developed_games
                            .into_iter()
                            .map(|e| GameDigest::from(e).compact())
                            .collect_vec(),
                        published: published_games
                            .into_iter()
                            .map(|e| GameDigest::from(e).compact())
                            .collect_vec(),
                    };
                    library::firestore::companies::write(&firestore, &company).await?;

                    let finish = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    println!("  -- {} msec", finish - start);
                }
                Err(status) => error!("{status}"),
            }
            i += 1;
        }

        println!("🦀🦀🦀 Deleting {} companies...", tbd.len());
        for company in tbd {
            match library::firestore::companies::delete(&firestore, company.id).await {
                Ok(()) => println!("Deleted {} ({})", company.name, company.id),
                Err(status) => error!("{status}"),
            }
        }
    }

    Ok(())
}

const BATCH_SIZE: u32 = 1000;
