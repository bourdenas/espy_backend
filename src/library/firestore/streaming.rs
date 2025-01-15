#[macro_export]
macro_rules! stream_games {
    (batch: $batch:expr, offset: $offset:expr, filter: $filter:expr, ordering: $ordering:expr, $processor:expr) => {
        $crate::stream_documents!(
            collection: "games",
            document: ::espy_backend::documents::GameEntry,
            batch: $batch,
            offset: $offset,
            filter: $filter,
            ordering: $ordering,
            $processor
        )
    };
    (filter: $filter:expr, ordering: $ordering:expr, $processor:expr) => {
        $crate::stream_documents!(
            collection: "games",
            document: ::espy_backend::documents::GameEntry,
            batch: 400,
            offset: 0,
            filter: $filter,
            ordering: $ordering,
            $processor
        )
    };
    (offset: $offset:expr, filter: $filter:expr, $processor:expr) => {
        $crate::stream_documents!(
            collection: "games",
            document: ::espy_backend::documents::GameEntry,
            batch: 400,
            offset: $offset,
            filter: $filter,
            ordering: [(
                ::firestore::path!(espy_backend::documents::GameEntry::release_date),
                ::firestore::FirestoreQueryDirection::Ascending,
            )],
            $processor
        )
    };
    (filter: $filter:expr, $processor:expr) => {
        $crate::stream_documents!(
            collection: "games",
            document: ::espy_backend::documents::GameEntry,
            batch: 400,
            offset: 0,
            filter: $filter,
            ordering: [(
                ::firestore::path!(espy_backend::documents::GameEntry::release_date),
                ::firestore::FirestoreQueryDirection::Ascending,
            )],
            $processor
        )
    };
}

#[macro_export]
macro_rules! stream_documents {
    (collection: $collection:literal, document: $Document:ty, batch: $batch:expr, offset: $offset:expr, filter: $filter:expr, ordering: $ordering:tt, $processor:expr) => {
        use futures::StreamExt;

        let mut i = $offset;
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
                            ::chrono::DateTime::from_timestamp(game_entry.release_date, 0).unwrap()
                        );

                        let start = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis();

                        if let Err(status) = $processor.process(&firestore, game_entry).await {
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

#[macro_export]
macro_rules! collect_games {
    (filter: $filter:expr) => {{
        use futures::TryStreamExt;

        let firestore = std::sync::Arc::new(::espy_backend::api::FirestoreApi::connect().await?);

        let mut documents: ::futures::stream::BoxStream<
            ::firestore::FirestoreResult<espy_backend::documents::GameEntry>,
        > = firestore
            .db()
            .fluent()
            .select()
            .from("games")
            .filter($filter)
            .order_by([(
                ::firestore::path!(espy_backend::documents::GameEntry::release_date),
                ::firestore::FirestoreQueryDirection::Ascending,
            )])
            .obj()
            .stream_query_with_errors()
            .await?;
        documents.try_collect::<Vec<GameEntry>>().await
    }};
}
