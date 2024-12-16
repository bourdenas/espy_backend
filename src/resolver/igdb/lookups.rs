use itertools::Itertools;
use tracing::instrument;

use crate::{
    documents::{
        IgdbAnnotation, IgdbCompany, IgdbGame, IgdbInvolvedCompany, IgdbWebsite, Image, ReleaseDate,
    },
    Status,
};

use super::{endpoints, request::post, IgdbConnection};

pub struct IgdbLookup<'a> {
    connection: &'a IgdbConnection,
}

impl<'a> IgdbLookup<'a> {
    pub fn new(connection: &'a IgdbConnection) -> Self {
        IgdbLookup { connection }
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_cover(&self, id: u64) -> Result<Option<Image>, Status> {
        let result: Vec<Image> = post(
            self.connection,
            endpoints::COVERS,
            &format!("fields *; where id={id};"),
        )
        .await?;

        Ok(result.into_iter().next())
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_company_logo(&self, id: u64) -> Result<Option<Image>, Status> {
        let result: Vec<Image> = post(
            self.connection,
            endpoints::COMPANY_LOGOS,
            &format!("fields *; where id={id};"),
        )
        .await?;

        Ok(result.into_iter().next())
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_game(&self, id: u64) -> Result<IgdbGame, Status> {
        let result: Vec<IgdbGame> = post(
            self.connection,
            endpoints::GAMES,
            &format!("fields *; where id={id};"),
        )
        .await?;

        match result.into_iter().next() {
            Some(igdb_game) => Ok(igdb_game),
            None => Err(Status::not_found(format!(
                "Failed to retrieve game with id={id}"
            ))),
        }
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_games(&self, ids: &[u64]) -> Result<Vec<IgdbGame>, Status> {
        post::<Vec<IgdbGame>>(
            self.connection,
            endpoints::GAMES,
            &format!(
                "fields *; where id = ({});",
                ids.into_iter()
                    .map(|id| id.to_string())
                    .collect_vec()
                    .join(",")
            ),
        )
        .await
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_artwork(&self, ids: &[u64]) -> Result<Vec<Image>, Status> {
        Ok(post(
            self.connection,
            endpoints::ARTWORKS,
            &format!(
                "fields *; where id = ({});",
                ids.iter().map(|id| id.to_string()).collect_vec().join(",")
            ),
        )
        .await?)
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_screenshots(&self, ids: &[u64]) -> Result<Vec<Image>, Status> {
        Ok(post(
            self.connection,
            endpoints::SCREENSHOTS,
            &format!(
                "fields *; where id = ({});",
                ids.iter().map(|id| id.to_string()).collect_vec().join(",")
            ),
        )
        .await?)
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_websites(&self, ids: &[u64]) -> Result<Vec<IgdbWebsite>, Status> {
        Ok(post(
            self.connection,
            endpoints::WEBSITES,
            &format!(
                "fields *; where id = ({});",
                ids.iter().map(|id| id.to_string()).collect_vec().join(",")
            ),
        )
        .await?)
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_collections(&self, ids: &[u64]) -> Result<Vec<IgdbAnnotation>, Status> {
        Ok(post::<Vec<IgdbAnnotation>>(
            self.connection,
            endpoints::COLLECTIONS,
            &format!(
                "fields *; where id = ({});",
                ids.iter().map(|id| id.to_string()).collect_vec().join(",")
            ),
        )
        .await?)
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_franchises(&self, ids: &[u64]) -> Result<Vec<IgdbAnnotation>, Status> {
        Ok(post::<Vec<IgdbAnnotation>>(
            self.connection,
            endpoints::FRANCHISES,
            &format!(
                "fields *; where id = ({});",
                ids.iter().map(|id| id.to_string()).collect_vec().join(",")
            ),
        )
        .await?)
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_involved_companies(
        &self,
        ids: &[u64],
    ) -> Result<Vec<IgdbInvolvedCompany>, Status> {
        Ok(post::<Vec<IgdbInvolvedCompany>>(
            self.connection,
            endpoints::INVOLVED_COMPANIES,
            &format!(
                "fields *; where id = ({});",
                ids.iter().map(|id| id.to_string()).collect_vec().join(",")
            ),
        )
        .await?)
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_companies(&self, ids: &[u64]) -> Result<Vec<IgdbCompany>, Status> {
        Ok(post::<Vec<IgdbCompany>>(
            self.connection,
            endpoints::COMPANIES,
            &format!(
                "fields *; where id = ({});",
                ids.iter().map(|id| id.to_string()).collect_vec().join(",")
            ),
        )
        .await?)
    }

    #[instrument(level = "info", skip(self))]
    pub async fn get_release_dates(&self, ids: &[u64]) -> Result<Vec<ReleaseDate>, Status> {
        Ok(post::<Vec<ReleaseDate>>(
            self.connection,
            endpoints::RELEASE_DATES,
            &format!(
                "fields category, date, status.name; where id = ({});",
                ids.iter().map(|id| id.to_string()).collect_vec().join(",")
            ),
        )
        .await?)
    }
}
