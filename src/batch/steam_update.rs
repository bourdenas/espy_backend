use std::collections::BTreeMap;

use chrono::Utc;
use clap::Parser;
use espy_backend::{
    api::{FirestoreApi, SteamDataApi},
    documents::{DayUpdates, GameEntry, Update},
    library, stream_games, Status,
};
use firestore::path;
use tracing::warn;

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "2015")]
    start_year: u64,

    #[clap(long, default_value = "0")]
    offset: u32,

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

    let mut steam_processor = SteamProcessor::new();
    stream_games!(
        offset: opts.offset,
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
        steam_processor
    );

    let firestore = FirestoreApi::connect().await?;
    for (date, updates) in steam_processor.updates_by_day {
        library::firestore::updates::write(&firestore, &DayUpdates { date, updates }).await?;
    }

    Ok(())
}

struct SteamProcessor {
    steam: SteamDataApi,
    updates_by_day: BTreeMap<String, Vec<Update>>,
}

impl SteamProcessor {
    fn new() -> Self {
        SteamProcessor {
            steam: SteamDataApi::new(),
            updates_by_day: BTreeMap::new(),
        }
    }

    async fn process(
        &mut self,
        firestore: &FirestoreApi,
        mut game_entry: GameEntry,
    ) -> Result<(), Status> {
        let steam_appid = format!("{}", game_entry.steam_appid.unwrap());

        let steam_data = match self.steam.retrieve_all_data(&steam_appid).await {
            Ok(steam_data) => steam_data,
            Err(status) => {
                warn!("retrieve_steam_data(): {status}");
                return Err(status);
            }
        };
        game_entry.add_steam_data(steam_data);

        library::firestore::games::write(firestore, &mut game_entry).await?;

        let now = Utc::now();
        if let Some(steam_data) = game_entry.steam_data {
            for item in steam_data.news {
                let date = chrono::DateTime::from_timestamp(item.date as i64, 0).unwrap();

                if date.signed_duration_since(now).num_days().abs() > 60 {
                    continue;
                }

                self.updates_by_day
                    .entry(date.format("%Y_%m_%d").to_string())
                    .or_insert_with(Vec::new)
                    .push(Update {
                        game_id: game_entry.id,
                        date: item.date,
                        url: item.url,
                        title: item.title,
                        contents: item.contents,
                        cover: match &game_entry.cover {
                            Some(image) => Some(image.image_id.clone()),
                            None => None,
                        },
                    });
            }
        }

        Ok(())
    }
}
