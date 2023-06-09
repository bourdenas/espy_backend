// use crate::espy;
use crate::Status;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct GogToken {
    #[serde(default)]
    pub access_token: String,

    #[serde(default)]
    refresh_token: String,

    #[serde(default)]
    expires_at: u64,

    #[serde(default)]
    user_id: String,

    #[serde(default)]
    session_id: String,
}

impl GogToken {
    /// Creates a GogToken for authenticating a user to the service. The
    /// authentication code is used to retrieve an access token that is used when
    /// calling any GOG API for retrieving user information.
    ///
    /// Retrieve GOG authentication code by loging in to:
    /// https://auth.gog.com/auth?client_id=46899977096215655&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code&layout=client2
    pub async fn from_oauth_code(oauth_code: &str) -> Result<Self, Status> {
        let params = format!("client_id={GOG_GALAXY_CLIENT_ID}&client_secret={GOG_GALAXY_SECRET}&grant_type=authorization_code&code={oauth_code}&redirect_uri={GOG_GALAXY_REDIRECT_URI}%2Ftoken");
        let uri = format!("{GOG_AUTH_HOST}/token?{params}");

        let resp = reqwest::get(&uri).await?.json::<GogAuthResponse>().await?;
        let internal_token = match resp {
            GogAuthResponse::Ok(internal_token) => internal_token,
            GogAuthResponse::Err(err) => {
                return Err(Status::new("Failed to retrieve GOG entries", err));
            }
        };

        Ok(internal_token.to_token())
    }

    /// Validates that the access token contained in a user's GogToken is still
    /// valid. If the access token is expired this will try to refresh it (without
    /// requiring any user interaction).
    ///
    /// Validation needs to happen before any request to GOG APIs. If an expired
    /// access token is used, then the user needs to manually authenticate with GOG,
    /// retrieve a new oauth code and provide it to the `create_from_oauth_code`
    /// function to produce a new GogToken.
    pub async fn validate(&mut self) -> Result<(), Status> {
        if self.access_token.is_empty() || self.refresh_token.is_empty() {
            return Err(Status::invalid_argument(&format!(
                "Invalid GogToken: {:?}",
                self
            )));
        }

        if self.is_fresh_token() {
            return Ok(());
        }

        let params = format!(
            "client_id={GOG_GALAXY_CLIENT_ID}&client_secret={GOG_GALAXY_SECRET}&grant_type=refresh_token&refresh_token={}&%2Ftoken",
            &self.refresh_token);
        let uri = format!("{GOG_AUTH_HOST}/token?{params}");

        let resp = reqwest::get(&uri).await?.json::<GogAuthResponse>().await?;
        let internal_token = match resp {
            GogAuthResponse::Ok(internal_token) => internal_token,
            GogAuthResponse::Err(err) => {
                return Err(Status::new("Failed to retrieve GOG entries", err));
            }
        };

        *self = internal_token.to_token();

        Ok(())
    }

    /// Returns true if the current user GOG access token has not expired yet.
    /// Typically, it is valid for 2 hours.
    fn is_fresh_token(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        now.as_secs() < self.expires_at
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum GogAuthResponse {
    Ok(GogTokenInternal),
    Err(GogAuthError),
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct GogAuthError {
    error: String,
    error_description: String,
}

use std::fmt;
impl fmt::Display for GogAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GOG Auth response error: '{}' -- {}",
            self.error, self.error_description
        )
    }
}

use std::error::Error;
impl Error for GogAuthError {}

/// GogTokenInternal struct used for serialisation/deserialisation to/from JSON
/// for requests to GOG auth servers.
#[derive(Debug, Serialize, Deserialize, Default)]
struct GogTokenInternal {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    user_id: String,
    session_id: String,
}

impl GogTokenInternal {
    /// Converts GogTokenInternal struct into a GogToken used for persistent
    /// storage.
    fn to_token(self) -> GogToken {
        GogToken {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: SystemTime::now()
                .checked_add(Duration::from_secs(self.expires_in))
                .unwrap()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            user_id: self.user_id,
            session_id: self.session_id,
        }
    }
}

const GOG_AUTH_HOST: &str = "https://auth.gog.com";
const GOG_GALAXY_CLIENT_ID: &str = "46899977096215655";
const GOG_GALAXY_SECRET: &str = "9d85c43b1482497dbbce61f6e4aa173a433796eeae2ca8c5f6129f2dc4de46d9";
const GOG_GALAXY_REDIRECT_URI: &str =
    "https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient";
