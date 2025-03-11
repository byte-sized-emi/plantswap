use std::fmt::Debug;

use async_trait::async_trait;
use bytes::Bytes;
use diesel::PgConnection;
use postgis_diesel::types::Point;
use serde::Serialize;

use crate::{config::AppConfig, models::Plant};

#[async_trait]
pub trait PlantRecogniser: Clone {
    type E: Debug;

    fn new(config: &AppConfig) -> Self;

    /// Returns all plants recognised, from best to worst match.
    /// Return 10 plants maximum. All plants returned must be in the
    /// database with those exact detailis (this method can insert them
    /// into the db incase of missing plants).
    async fn analyze_plant(&self, db: &mut PgConnection, info: &PlantRecognitionInfo)
        -> Result<Vec<RankedPlant>, Self::E>;
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct RankedPlant {
    pub plant: Plant,
    pub score: f32,
}

impl RankedPlant {
    pub fn new(plant: Plant, score: f32) -> Self {
        Self { plant, score }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PlantRecognitionInfo {
    // image data + filename
    pub images: Vec<(Bytes, String)>,
    pub location: Option<Point>,
}

pub mod plantnet {
    use std::sync::Arc;

    use axum::async_trait;
    use diesel::{ExpressionMethods, Insertable, OptionalExtension, PgConnection, SelectableHelper};
    use reqwest::{multipart::{Form, Part}, Url};
    use serde::Deserialize;

    use crate::{config::AppConfig, models::{InsertPlant, Plant}};

    use super::*;

    #[derive(Debug, Clone)]
    pub struct PlantNetRecogniser {
        http_client: Arc<reqwest::Client>,
        base_url: Url,
        apikey: String,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum PlantNetError {
        #[error("Reqwest error: {0}")]
        Reqwest(#[from] reqwest::Error),
        #[error("diesel/db error: {0}")]
        Diesel(#[from] diesel::result::Error),
    }

    impl PlantNetRecogniser {
        pub fn from_parts(base_url: Url, apikey: String) -> Self {
            let http_client = reqwest::Client::builder()
                .https_only(true)
                .build()
                .expect("failed to built plantnet reqwest client");

            let http_client = Arc::new(http_client);

            Self { http_client, base_url, apikey }
        }
    }

    #[async_trait]
    impl PlantRecogniser for PlantNetRecogniser {
        type E = PlantNetError;

        fn new(config: &AppConfig) -> Self {
            let base_url = Url::parse(config.plantnet_api_url())
                .expect("invalid plantnet base url");

            PlantNetRecogniser::from_parts(
                base_url,
                config.plantnet_api_key().to_string()
            )
        }

        async fn analyze_plant(&self, db: &mut PgConnection, info: &PlantRecognitionInfo) -> Result<Vec<RankedPlant>, Self::E> {
            let url = self.base_url.join("identity/all").unwrap();

            let mut body = Form::new();

            for (image, image_name) in &info.images {
                let part = Part::stream(image.clone()).file_name(image_name.clone());
                body = body.part("images", part);
            }

            let response: RecogniseResponse = self.http_client.post(url)
                .query(&[
                    ("nb-results", "10"),
                    ("lang", "en"),
                    ("api-key", &self.apikey)
                    ])
                .multipart(body)
                .send().await?
                .json().await?;

            let mut plants = Vec::new();
            for plant in response.results {
                let db_plant = insert_or_load_plant(db, &plant)?;
                plants.push(RankedPlant::new(db_plant, plant.score));
            }

            Ok(plants)
        }
    }

    fn insert_or_load_plant(db: &mut PgConnection, res: &RecogniseResult) -> Result<Plant, diesel::result::Error> {
        use crate::schema::plants::dsl::*;
        use diesel::{QueryDsl, RunQueryDsl};

        let plant_in_db = plants.filter(powo_id.eq(&res.powo.id))
            .select(Plant::as_select())
            .get_result(db).optional()?;

        if let Some(plant_in_db) = plant_in_db {
            Ok(plant_in_db)
        } else {
            let insert_plant = InsertPlant {
                powo_id: res.powo.id.clone(),
                gbif_id: res.gbif.id.parse().ok(),
                human_name: res.species.common_names.first().unwrap().to_string(),
                species: res.species.scientific_name_without_author.clone(),
                location: None,
                produces_fruit: None,
                description: "".to_string(),
            };

            insert_plant.insert_into(plants)
                .returning(Plant::as_returning())
                .get_result(db)
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct RecogniseResponse {
        results: Vec<RecogniseResult>
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct RecogniseResult {
        score: f32,
        species: RecognisedSpecies,
        gbif: Id,
        powo: Id,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct Id {
        id: String
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct RecognisedSpecies {
        scientific_name_without_author: String,
        common_names: Vec<String>,
    }
}
