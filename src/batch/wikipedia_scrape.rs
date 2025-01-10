use clap::Parser;
use espy_backend::{
    api::{self, FirestoreApi},
    documents::{GameEntry, WebsiteAuthority, WikipediaData},
    library, stream_games, Status, Tracing,
};
use firestore::{struct_path::path, FirestoreQueryDirection};
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

    #[clap(long, default_value = "1970")]
    start_year: u64,
}

#[tokio::main]
async fn main() -> Result<(), Status> {
    Tracing::setup("utils/wikipedia_scrape")?;

    let opts: Opts = Opts::parse();

    // If an `id` argument is provided only scrape the specific GameEntry.
    if let Some(id) = opts.id {
        let result = scrape(&api::FirestoreApi::connect().await?, id, &opts.kw_source).await;
        println!("result = {:?}", result);
        return Ok(());
    }

    let start = chrono::DateTime::parse_from_str(
        &format!("{}-01-01 00:00:00 +0000", opts.start_year),
        "%Y-%m-%d %H:%M:%S %z",
    )
    .expect("Failed to parse start date")
    .timestamp();

    let wikipedia_processor = WikipediaProcessor::new(&opts.kw_source);
    stream_games!(
        filter: |q| {
            q.for_any([
                q.field(path!(GameEntry::release_date))
                    .greater_than_or_equal(start),
                q.field(path!(GameEntry::release_date)).equal(0),
            ])
        },
        ordering: [(
            path!(GameEntry::release_date),
            FirestoreQueryDirection::Ascending,
        )],
        wikipedia_processor
    );

    Ok(())
}

struct WikipediaProcessor {
    wikipedia: api::Wikipedia,
}

impl WikipediaProcessor {
    fn new(kw_source: &str) -> Self {
        WikipediaProcessor {
            wikipedia: api::Wikipedia::new(kw_source).unwrap(),
        }
    }

    async fn process(
        &self,
        firestore: &FirestoreApi,
        game_entry: &mut GameEntry,
    ) -> Result<(), Status> {
        let website = game_entry
            .websites
            .iter()
            .find(|e| matches!(e.authority, WebsiteAuthority::Wikipedia));
        if let Some(website) = website {
            let response = self
                .wikipedia
                .scrape(game_entry.id, game_entry.name.clone(), &website.url)
                .await;
            match response {
                Ok(wiki_data) => {
                    if !wiki_data.is_empty() {
                        library::firestore::wikipedia::write(&firestore, game_entry.id, &wiki_data)
                            .await?;
                    }
                }
                Err(status) => error!("{status}"),
            }
        }

        Ok(())
    }
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
