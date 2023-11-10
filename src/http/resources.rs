use crate::{
    api::{FirestoreApi, IgdbApi},
    util,
};
use std::{convert::Infallible, sync::Arc};
use warp::{self, Filter};

pub fn with_igdb(
    igdb: Arc<IgdbApi>,
) -> impl Filter<Extract = (Arc<IgdbApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&igdb))
}

pub fn with_firestore(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (Arc<FirestoreApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&firestore))
}

pub fn with_keys(
    keys: Arc<util::keys::Keys>,
) -> impl Filter<Extract = (Arc<util::keys::Keys>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&keys))
}
