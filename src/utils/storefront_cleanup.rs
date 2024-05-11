use clap::Parser;
use espy_backend::{
    api::FirestoreApi,
    documents::{Library, LibraryEntry, StoreEntry},
    library::firestore,
    Status, Tracing,
};

#[derive(Parser)]
struct Opts {
    /// Espy user name for managing a game library.
    #[clap(short, long, default_value = "")]
    user: String,
}

/// Verifies that all game ids that exist in in /users/{id}/strorefront/{store}
/// document are also included in the user library of matched or failed entries.
/// If a game id is missing from the library it is deleted in order to be picked
/// up again for recon on the next storefront sync.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Tracing::setup("util/storefront_cleanup")?;

    let opts: Opts = Opts::parse();

    let firestore = FirestoreApi::connect().await?;

    let user_library = firestore::library::read(&firestore, &opts.user).await?;
    let failed = firestore::failed::read(&firestore, &opts.user)
        .await?
        .entries;

    storefront_cleanup(&firestore, &opts.user, &user_library, &failed, "gog")
        .await
        .expect("Failed to cleanup GOG");
    storefront_cleanup(&firestore, &opts.user, &user_library, &failed, "steam")
        .await
        .expect("Failed to cleanup Steam");

    Ok(())
}

async fn storefront_cleanup(
    firestore: &FirestoreApi,
    user_id: &str,
    user_library: &Library,
    user_failed: &[StoreEntry],
    storefront_name: &str,
) -> Result<(), Status> {
    let mut storefront =
        match firestore::storefront::read(&firestore, user_id, storefront_name).await {
            Ok(doc) => doc,
            Err(Status::NotFound(_)) => return Ok(()),
            Err(status) => return Err(status),
        };

    let mut missing = vec![];
    for store_entry in &storefront.games {
        let iter = user_library
            .entries
            .iter()
            .find(|entry| find_store_entry(entry, &store_entry.id, storefront_name));
        if let None = iter {
            let iter = user_failed.iter().find(|entry| {
                entry.id == store_entry.id && entry.storefront_name == storefront_name
            });

            if let None = iter {
                missing.push(store_entry.clone());
            }
        }
    }
    println!(
        "Missing {} {storefront_name} games from user library\nids={:?}",
        missing.len(),
        missing
    );
    storefront
        .games
        .retain(|e| !missing.iter().all(|m| m.id != e.id));
    firestore::storefront::write(firestore, user_id, &storefront).await?;

    Ok(())
}

fn find_store_entry(library_entry: &LibraryEntry, id: &str, store_name: &str) -> bool {
    library_entry
        .store_entries
        .iter()
        .find(|entry| entry.id == id && entry.storefront_name == store_name)
        .is_some()
}
