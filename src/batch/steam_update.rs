use clap::Parser;
use espy_backend::{
    api::{FirestoreApi, SteamDataApi, SteamScrape},
    documents::GameEntry,
    library, stream_games, Status,
};
use firestore::{path, FirestoreQueryDirection};

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "2015")]
    start_year: u64,

    #[clap(long, default_value = "false")]
    cleanup: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Tracing::setup("batch/steam_update")?;

    let opts: Opts = Opts::parse();

    let start = chrono::DateTime::parse_from_str(
        &format!("{}-01-01 00:00:00 +0000", opts.start_year),
        "%Y-%m-%d %H:%M:%S %z",
    )
    .expect("Failed to parse start date")
    .timestamp();

    let steam_processor = SteamProcessor::new();
    stream_games!(
        batch: 400,
        filter: |q| {
            q.for_all([
                q.field(path!(GameEntry::steam_appid)).is_not_null(),
                q.for_any([
                    q.field(path!(GameEntry::release_date))
                        .greater_than_or_equal(start),
                    q.field(path!(GameEntry::release_date)).equal(0),
                ]),
            ])
        },
        ordering: [(
            path!(GameEntry::release_date),
            FirestoreQueryDirection::Ascending,
        )],
        steam_processor
    );

    Ok(())
}

struct SteamProcessor {
    steam: SteamDataApi,
}

impl SteamProcessor {
    fn new() -> Self {
        SteamProcessor {
            steam: SteamDataApi::new(),
        }
    }

    async fn process(
        &self,
        firestore: &FirestoreApi,
        game_entry: &mut GameEntry,
    ) -> Result<(), Status> {
        let steam_appid = format!("{}", game_entry.steam_appid.unwrap());

        let steam_data = self.steam.retrieve_steam_data(&steam_appid).await?;
        game_entry.add_steam_data(steam_data);

        let website = format!("https://store.steampowered.com/app/{steam_appid}/");
        let scraped_data = SteamScrape::scrape(&website).await?;
        if let Some(steam_data) = &mut game_entry.steam_data {
            steam_data.user_tags = scraped_data.user_tags;
        }

        library::firestore::games::write(firestore, game_entry).await?;

        Ok(())
    }
}
