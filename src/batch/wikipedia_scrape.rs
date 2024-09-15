use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::DateTime;
use clap::Parser;
use espy_backend::{
    api,
    documents::{GameEntry, WebsiteAuthority, WikipediaData},
    library, Status, Tracing,
};
use firestore::{struct_path::path, FirestoreQueryDirection, FirestoreResult};
use futures::{stream::BoxStream, StreamExt};
use tracing::error;

#[derive(Parser)]
struct Opts {
    /// JSON file that contains application keys for espy service.
    #[clap(long, default_value = "keys.json")]
    key_store: String,

    #[clap(long, default_value = "wikipedia_keywords.txt")]
    kw_source: String,

    #[clap(long)]
    id: Option<u64>,

    #[clap(long, default_value = "0")]
    cursor: u64,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    Tracing::setup("utils/wikipedia_scrape")?;

    let opts: Opts = Opts::parse();

    if let Some(id) = opts.id {
        let result = scrape(&api::FirestoreApi::connect().await?, id, &opts.kw_source).await;
        println!("result = {:?}", result);
        return Ok(());
    }

    let wikipedia = api::Wikipedia::new(&opts.kw_source).unwrap();
    let mut cursor = opts.cursor;
    let mut i = 0;
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    while i % BATCH_SIZE == 0 {
        let firestore = Arc::new(api::FirestoreApi::connect().await?);

        let mut game_entries: BoxStream<FirestoreResult<GameEntry>> = firestore
            .db()
            .fluent()
            .select()
            .from("games")
            // .start_at(FirestoreQueryCursor::AfterValue(vec![(&cursor).into()]))
            .filter(|q| {
                q.for_all([
                    q.field(path!(GameEntry::release_date)).less_than(today),
                    q.field(path!(GameEntry::release_date)).greater_than(cursor),
                ])
            })
            .order_by([(
                path!(GameEntry::release_date),
                FirestoreQueryDirection::Ascending,
            )])
            .limit(BATCH_SIZE)
            .obj()
            .stream_query_with_errors()
            .await?;

        while let Some(game_entry) = game_entries.next().await {
            match game_entry {
                Ok(game_entry) => {
                    cursor = game_entry.release_date as u64;

                    println!(
                        "#{i} -- {} -- id={} -- release={} ({})",
                        game_entry.name,
                        game_entry.id,
                        game_entry.release_date,
                        DateTime::from_timestamp_millis(game_entry.release_date * 1000).unwrap()
                    );

                    let start = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();

                    let website = game_entry
                        .websites
                        .iter()
                        .find(|e| matches!(e.authority, WebsiteAuthority::Wikipedia));
                    if let Some(website) = website {
                        let response = wikipedia
                            .scrape(game_entry.id, game_entry.name, &website.url)
                            .await;
                        match response {
                            Ok(wiki_data) => {
                                if !wiki_data.is_empty() {
                                    library::firestore::wikipedia::write(
                                        &firestore,
                                        game_entry.id,
                                        &wiki_data,
                                    )
                                    .await?;
                                }
                            }
                            Err(status) => error!("{status}"),
                        }
                    }

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
    }

    Ok(())
}

async fn scrape(
    firestore: &api::FirestoreApi,
    id: u64,
    kw_source: &str,
) -> Result<WikipediaData, Status> {
    let wikipedia = api::Wikipedia::new(kw_source).unwrap();

    match library::firestore::games::read(firestore, id).await {
        Ok(game_entry) => {
            match game_entry
                .websites
                .iter()
                .find(|e| matches!(e.authority, WebsiteAuthority::Wikipedia))
            {
                Some(website) => match wikipedia.scrape(id, game_entry.name, &website.url).await {
                    Ok(wiki_data) => {
                        if !wiki_data.is_empty() {
                            library::firestore::wikipedia::write(
                                &firestore,
                                game_entry.id,
                                &wiki_data,
                            )
                            .await?;
                        }
                        Ok(wiki_data)
                    }
                    Err(status) => Err(status),
                },
                None => Err(Status::invalid_argument(format!(
                    "'{}' missing a wikipedia link",
                    game_entry.name
                ))),
            }
        }
        Err(status) => Err(status),
    }
}

const BATCH_SIZE: u32 = 400;
