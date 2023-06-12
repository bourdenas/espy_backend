use crate::{
    documents::{Genre, Keyword},
    Status,
};
use tracing::instrument;

use super::{
    backend::post,
    docs::{IgdbCollection, IgdbCompany, IgdbExternalGame},
    resolve::{
        COLLECTIONS_ENDPOINT, COMPANIES_ENDPOINT, EXTERNAL_GAMES_ENDPOINT, FRANCHISES_ENDPOINT,
        GAMES_ENDPOINT, GENRES_ENDPOINT, KEYWORDS_ENDPOINT,
    },
    IgdbApi, IgdbGame,
};

pub struct IgdbBatchApi {
    service: IgdbApi,
}

impl IgdbBatchApi {
    pub fn new(service: IgdbApi) -> IgdbBatchApi {
        IgdbBatchApi { service }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_igdb_games(
        &self,
        updated_since: u64,
        offset: u64,
    ) -> Result<Vec<IgdbGame>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<IgdbGame>>(
            &connection,
            GAMES_ENDPOINT,
            &format!("fields *; where (platforms = (6,13,14) | platforms = null) & updated_at >= {updated_since} & (follows > 0 | hypes > 0) & (category = 0 | category = 1 | category = 2 | category = 4 | category = 8 | category = 9); limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_igdb_games_by_collection(
        &self,
        collection_id: u64,
        offset: u64,
    ) -> Result<Vec<IgdbGame>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<IgdbGame>>(
            &connection,
            GAMES_ENDPOINT,
            &format!("fields *; where platforms = (6,13,14) & collection = {collection_id} & (category = 0 | category = 1 | category = 2 | category = 4 | category = 8 | category = 9); limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_igdb_games_by_franchise(
        &self,
        franchise_id: u64,
        offset: u64,
    ) -> Result<Vec<IgdbGame>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<IgdbGame>>(
            &connection,
            GAMES_ENDPOINT,
            &format!("fields *; where platforms = (6,13,14) & (franchise = {franchise_id} | franchises = ({franchise_id})) & (category = 0 | category = 1 | category = 2 | category = 4 | category = 8 | category = 9); limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_collections(
        &self,
        updated_since: u64,
        offset: u64,
    ) -> Result<Vec<IgdbCollection>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<IgdbCollection>>(
            &connection,
            COLLECTIONS_ENDPOINT,
            &format!("fields *; where updated_at >= {updated_since}; limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn search_collection(&self, slug: &str) -> Result<Vec<IgdbCollection>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<IgdbCollection>>(
            &connection,
            COLLECTIONS_ENDPOINT,
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
        let connection = self.service.connection()?;
        post::<Vec<IgdbCollection>>(
            &connection,
            FRANCHISES_ENDPOINT,
            &format!("fields *; limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn search_franchises(&self, slug: &str) -> Result<Vec<IgdbCollection>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<IgdbCollection>>(
            &connection,
            FRANCHISES_ENDPOINT,
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
        let connection = self.service.connection()?;
        post::<Vec<IgdbCompany>>(
            &connection,
            COMPANIES_ENDPOINT,
            &format!("fields *; where updated_at >= {updated_since}; limit 500; offset {offset};"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn search_company(&self, slug: &str) -> Result<Vec<IgdbCompany>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<IgdbCompany>>(
            &connection,
            COMPANIES_ENDPOINT,
            &format!("fields *; where slug = \"{slug}\"; limit 500;"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_genres(&self) -> Result<Vec<Genre>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<Genre>>(
            &connection,
            GENRES_ENDPOINT,
            &format!("fields *; limit 500;"),
        )
        .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn collect_keywords(
        &self,
        updated_since: u64,
        offset: u64,
    ) -> Result<Vec<Keyword>, Status> {
        let connection = self.service.connection()?;
        post::<Vec<Keyword>>(
            &connection,
            KEYWORDS_ENDPOINT,
            &format!("fields *; where updated_at >= {updated_since}; limit 500; offset {offset};"),
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

        let connection = self.service.connection()?;
        post::<Vec<IgdbExternalGame>>(
            &connection,
            EXTERNAL_GAMES_ENDPOINT,
            &format!(
                "fields *; sort uid; where category = {category}; limit 500; offset {offset};"
            ),
        )
        .await
    }
}
