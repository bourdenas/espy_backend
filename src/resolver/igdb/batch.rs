use crate::{
    documents::{IgdbCollection, IgdbCompany, IgdbExternalGame, IgdbGame, IgdbGenreType, Keyword},
    Status,
};
use tracing::instrument;

use super::{endpoints, request::post, IgdbConnection};

pub struct IgdbBatchApi {
    connection: IgdbConnection,
}

impl IgdbBatchApi {
    pub fn new(connection: IgdbConnection) -> IgdbBatchApi {
        IgdbBatchApi { connection }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_igdb_games(
        &self,
        updated_since: u64,
        offset: u64,
    ) -> Result<Vec<IgdbGame>, Status> {
        post::<Vec<IgdbGame>>(
            &self.connection,
            endpoints::GAMES,
            &format!("fields *; where (platforms = (6,13) | platforms = null) & updated_at >= {updated_since} & (follows > 0 | hypes > 0) & (category = 0 | category = 1 | category = 2 | category = 4 | category = 8 | category = 9); limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_igdb_games_by_collection(
        &self,
        collection_id: u64,
        offset: u64,
    ) -> Result<Vec<IgdbGame>, Status> {
        post::<Vec<IgdbGame>>(
            &self.connection,
            endpoints::GAMES,
            &format!("fields *; where platforms = (6,13) & collection = {collection_id} & (category = 0 | category = 1 | category = 2 | category = 4 | category = 8 | category = 9); limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_igdb_games_by_franchise(
        &self,
        franchise_id: u64,
        offset: u64,
    ) -> Result<Vec<IgdbGame>, Status> {
        post::<Vec<IgdbGame>>(
            &self.connection,
            endpoints::GAMES,
            &format!("fields *; where platforms = (6,13) & (franchise = {franchise_id} | franchises = ({franchise_id})) & (category = 0 | category = 1 | category = 2 | category = 4 | category = 8 | category = 9); limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_collections(
        &self,
        updated_since: u64,
        offset: u64,
    ) -> Result<Vec<IgdbCollection>, Status> {
        post::<Vec<IgdbCollection>>(
            &self.connection,
            endpoints::COLLECTIONS,
            &format!("fields *; where updated_at >= {updated_since}; limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn search_collection(&self, slug: &str) -> Result<Vec<IgdbCollection>, Status> {
        post::<Vec<IgdbCollection>>(
            &self.connection,
            endpoints::COLLECTIONS,
            &format!("fields *; where slug = \"{slug}\"; limit 500;"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_franchises(
        &self,
        _updated_since: u64,
        offset: u64,
    ) -> Result<Vec<IgdbCollection>, Status> {
        post::<Vec<IgdbCollection>>(
            &self.connection,
            endpoints::FRANCHISES,
            &format!("fields *; limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn search_franchises(&self, slug: &str) -> Result<Vec<IgdbCollection>, Status> {
        post::<Vec<IgdbCollection>>(
            &self.connection,
            endpoints::FRANCHISES,
            &format!("fields *; where slug = \"{slug}\"; limit 500;"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_companies(
        &self,
        updated_since: u64,
        offset: u64,
    ) -> Result<Vec<IgdbCompany>, Status> {
        post::<Vec<IgdbCompany>>(
            &self.connection,
            endpoints::COMPANIES,
            &format!("fields *; where updated_at >= {updated_since}; limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn search_company(&self, slug: &str) -> Result<Vec<IgdbCompany>, Status> {
        post::<Vec<IgdbCompany>>(
            &self.connection,
            endpoints::COMPANIES,
            &format!("fields *; where slug = \"{slug}\"; limit 500;"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_genres(&self) -> Result<Vec<IgdbGenreType>, Status> {
        post::<Vec<IgdbGenreType>>(
            &self.connection,
            endpoints::GENRES,
            &format!("fields *; limit 500;"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_keywords(&self, offset: u64) -> Result<Vec<Keyword>, Status> {
        post::<Vec<Keyword>>(
            &self.connection,
            endpoints::KEYWORDS,
            &format!("fields *; limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_external_games(
        &self,
        external_source: &str,
        offset: u64,
    ) -> Result<Vec<IgdbExternalGame>, Status> {
        let category: u8 = match external_source {
            "steam" => 1,
            "gog" => 5,
            "egs" => 26,
            _ => {
                return Err(Status::invalid_argument(format!(
                    "Unrecognised source: {external_source}"
                )));
            }
        };

        post::<Vec<IgdbExternalGame>>(
            &self.connection,
            endpoints::EXTERNAL_GAMES,
            &format!("fields *; where category = {category}; limit 500; offset {offset};"),
        )
        .await
    }
}
