use clap::Parser;
use espy_backend::{
    api::FirestoreApi, documents::GameEntry, library, resolver::ResolveApi, stream_games, Status,
    Tracing,
};
use firestore::path;

/// Espy util for refreshing IGDB and Steam data for GameEntries.
#[derive(Parser)]
struct Opts {
    /// URL of the resolver backend.
    #[clap(
        long,
        default_value = "https://resolver-478783154654.europe-west1.run.app"
    )]
    resolver_backend: String,

    #[clap(long, default_value = "1970")]
    start_year: u64,

    #[clap(long, default_value = "0")]
    offset: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/refresh_game_entries")?;

    let opts: Opts = Opts::parse();

    let start = chrono::DateTime::parse_from_str(
        &format!("{}-01-01 00:00:00 +0000", opts.start_year),
        "%Y-%m-%d %H:%M:%S %z",
    )
    .expect("Failed to parse start date")
    .timestamp();

    let refresh_processor = RefreshProcessor::new(opts.resolver_backend);
    stream_games!(
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
        refresh_processor
    );

    Ok(())
}

struct RefreshProcessor {
    resolver: ResolveApi,
}

impl RefreshProcessor {
    fn new(resolver_backend: String) -> Self {
        RefreshProcessor {
            resolver: ResolveApi::new(resolver_backend),
        }
    }

    async fn process(
        &self,
        firestore: &FirestoreApi,
        game_entry: &mut GameEntry,
    ) -> Result<(), Status> {
        let mut game_entry = self.resolver.retrieve(game_entry.id).await?;
        library::firestore::games::write(firestore, &mut game_entry).await?;
        Ok(())
    }
}
