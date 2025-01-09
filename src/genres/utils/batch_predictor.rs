use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    documents::{GameEntry, Genre},
    genres::GenrePredictor,
    library::{self, firestore::wikipedia},
    stream_games, Status, Tracing,
};
use firestore::{path, FirestoreQueryDirection};
use tracing::warn;

#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "1970")]
    start_year: u64,

    #[clap(long, default_value = "http://localhost:8080")]
    predictor_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("utils/batch_predictor")?;

    let opts: Opts = Opts::parse();

    let start = chrono::DateTime::parse_from_str(
        &format!("{}-01-01 00:00:00 +0000", opts.start_year),
        "%Y-%m-%d %H:%M:%S %z",
    )
    .expect("Failed to parse start date")
    .timestamp();

    stream_games!(
        batch: 500,
        filter: |q| {
            q.for_any([
                q.field(path!(GameEntry::release_date)).greater_than_or_equal(start),
                q.field(path!(GameEntry::release_date)).equal(0),
            ])
        },
        ordering: [(
            path!(GameEntry::release_date),
            FirestoreQueryDirection::Ascending,
        )],
        predict_genre
    );

    Ok(())
}

async fn predict_genre(firestore: &FirestoreApi, game_entry: &mut GameEntry) -> Result<(), Status> {
    let wiki_data = match wikipedia::read(&firestore, game_entry.id).await {
        Ok(wiki_data) => Some(wiki_data),
        Err(Status::NotFound(_)) => None,
        Err(status) => panic!("{status}"),
    };

    let mut parent = match &game_entry.parent {
        Some(parent) => match library::firestore::games::read(&firestore, parent.id).await {
            Ok(parent) => Some(parent),
            Err(Status::NotFound(_)) => {
                warn!(
                    "Missing parent '{}' ({}) for entry '{}' ({})",
                    parent.name, parent.id, game_entry.name, game_entry.id
                );
                None
            }
            Err(status) => {
                warn!(
                    "Failed to retrieve parent for entry '{}' ({}): {status}",
                    game_entry.name, game_entry.id
                );
                None
            }
        },
        None => None,
    };
    let parent_wiki_data = match &game_entry.parent {
        Some(parent) => match wikipedia::read(&firestore, parent.id).await {
            Ok(wiki_data) => Some(wiki_data),
            Err(Status::NotFound(_)) => None,
            Err(status) => panic!("{status}"),
        },
        None => None,
    };

    let predictor = GenrePredictor::new("http://localhost:8080".to_owned());
    let espy_genres = predictor
        .predict(&game_entry, wiki_data, parent.as_ref(), parent_wiki_data)
        .await?;

    if !espy_genres.is_empty() {
        println!("  predicted genres={:?}", &espy_genres);
        game_entry.espy_genres = espy_genres.clone();

        if let Some(parent) = &mut parent {
            parent.espy_genres = espy_genres.clone();
        }

        library::firestore::genres::write(
            &firestore,
            &Genre {
                game_id: game_entry.id,
                espy_genres,
            },
        )
        .await?;

        library::firestore::games::write(&firestore, game_entry).await?;
        if let Some(parent) = &mut parent {
            library::firestore::games::write(&firestore, parent).await?;
        }
    }

    Ok(())
}
