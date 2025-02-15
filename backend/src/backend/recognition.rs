use std::fmt::Display;

use async_trait::async_trait;
use bytes::Bytes;
use diesel::PgConnection;
use postgis_diesel::types::Point;

use crate::{config::AppConfig, models::Plant};

#[async_trait]
pub trait PlantRecogniser: Sized {
    type E: Display;

    /// Returns all plants recognised, from best to worst match.
    /// Return 10 plants maximum. All plants returned must be in the
    /// database with those exact detailis (this method can insert them
    /// into the db incase of missing plants).
    async fn analyze_plant(&mut self, db: &mut PgConnection, info: PlantRecognitionInfo)
        -> Result<Vec<Plant>, Self::E>;
}

#[derive(Debug, PartialEq)]
pub struct PlantRecognitionInfo {
    images: Vec<Bytes>,
    location: Option<Point>,
}

#[derive(Debug)]
struct PlantNetRecogniser {
    apikey: String,
}

impl PlantNetRecogniser {
    fn new(config: &AppConfig) -> Self {
        Self { apikey: config.plantnet_api_key().to_string() }
    }
}
#[async_trait]
impl PlantRecogniser for PlantNetRecogniser {
    type E = String;

    async fn analyze_plant(&mut self, db: &mut PgConnection, info: PlantRecognitionInfo) -> Result<Vec<Plant>, Self::E> {
        todo!()
    }
}

