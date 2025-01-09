#[macro_export]
macro_rules! stream_games {
    (batch: $batch:expr, filter: $filter:expr, ordering: $ordering:expr, $process:expr) => {
        $crate::stream_documents!(collection: "games", document: ::espy_backend::documents::GameEntry, batch: $batch, filter: $filter, ordering: $ordering, $process)
    };
    (filter: $filter:expr, ordering: $ordering:expr, $process:expr) => {
        $crate::stream_documents!(collection: "games", document: ::espy_backend::documents::GameEntry, batch: 400, filter: $filter, ordering: $ordering, $process)
    };
}

#[macro_export]
macro_rules! stream_documents {
    (collection: $collection:literal, document: $Document:ty, batch: $batch:expr, filter: $filter:expr, ordering: $ordering:expr, $process:expr) => {
        use futures::StreamExt;

        let mut i = 0;
        while i % $batch == 0 {
            let firestore =
                std::sync::Arc::new(::espy_backend::api::FirestoreApi::connect().await?);

            let mut documents: ::futures::stream::BoxStream<
                ::firestore::FirestoreResult<$Document>,
            > = firestore
                .db()
                .fluent()
                .select()
                .from($collection)
                .filter($filter)
                .order_by($ordering)
                .offset(i)
                .limit($batch)
                .obj()
                .stream_query_with_errors()
                .await?;

            while let Some(game_entry) = documents.next().await {
                match game_entry {
                    Ok(mut game_entry) => {
                        println!(
                            "#{i} -- {} -- id={} -- release={} ({})",
                            game_entry.name,
                            game_entry.id,
                            game_entry.release_date,
                            ::chrono::DateTime::from_timestamp_millis(
                                game_entry.release_date * 1000
                            )
                            .unwrap()
                        );

                        let start = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis();

                        if let Err(status) = $process(&firestore, &mut game_entry).await {
                            ::tracing::error!("{status}");
                        }

                        let finish = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis();
                        println!("  -- {} msec", finish - start);
                    }
                    Err(e) => ::tracing::error!("{e}"),
                }
                i += 1;
            }
        }
    };
}
