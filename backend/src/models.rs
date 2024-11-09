use std::io::Write;
use axum_typed_multipart::TryFromField;
use diesel::{deserialize::{self, FromSql, FromSqlRow}, expression::AsExpression, pg::{Pg, PgValue}, prelude::*, serialize::{self, IsNull, Output, ToSql}};
use postgis_diesel::types::Point;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, FromSqlRow, AsExpression, Serialize, Deserialize, TryFromField)]
#[diesel(sql_type = crate::schema::sql_types::ListingType)]
pub enum ListingType {
    Selling,
    Buying,
}

impl ToSql<crate::schema::sql_types::ListingType, Pg> for ListingType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            ListingType::Selling => out.write_all(b"selling")?,
            ListingType::Buying => out.write_all(b"buying")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::ListingType, Pg> for ListingType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"buying" => Ok(ListingType::Buying),
            b"selling" => Ok(ListingType::Selling),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, FromSqlRow, AsExpression, Serialize, Deserialize)]
#[diesel(sql_type = crate::schema::sql_types::PlantLocation)]
pub enum PlantLocation {
    Outdoor,
    Indoor,
}

impl ToSql<crate::schema::sql_types::PlantLocation, Pg> for PlantLocation {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            PlantLocation::Outdoor => out.write_all(b"outdoor")?,
            PlantLocation::Indoor => out.write_all(b"indoor")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::PlantLocation, Pg> for PlantLocation {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"outdoor" => Ok(PlantLocation::Outdoor),
            b"indoor" => Ok(PlantLocation::Indoor),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, PartialEq)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: Uuid,
    pub location: Option<Point>,
}

#[derive(Insertable, Serialize, Deserialize, Debug, PartialEq)]
#[diesel(table_name = crate::schema::listings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertListing {
    pub title: String,
    pub description: String,
    pub author: Uuid,
    pub listing_type: ListingType,
    pub tradeable: Option<bool>,
    pub thumbnail: Uuid
}

#[derive(Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize, Debug, PartialEq)]
#[diesel(table_name = crate::schema::listings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Image, foreign_key = thumbnail))]
#[diesel(belongs_to(Plant, foreign_key = identified_plant))]
pub struct Listing {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub insertion_date: chrono::NaiveDateTime,
    pub author: Uuid,
    pub listing_type: ListingType,
    pub thumbnail: Uuid,
    pub tradeable: bool,
    pub identified_plant: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, PartialEq)]
#[diesel(table_name = crate::schema::images)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(file_key))]
pub struct Image {
    pub file_key: Uuid,
    pub uploaded_by_user: Option<Uuid>,
    pub upload_date: chrono::NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, Debug, PartialEq)]
#[diesel(table_name = crate::schema::images)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertImage {
    pub file_key: Uuid,
    pub uploaded_by_user: Option<Uuid>,
}

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, PartialEq)]
#[diesel(table_name = crate::schema::plants)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Plant {
    pub id: i32,
    pub human_name: String,
    pub species: String,
    pub location: Option<PlantLocation>,
    pub produces_fruit: Option<bool>,
    pub description: String,
}

#[derive(Identifiable, Queryable, Selectable, Insertable, PartialEq)]
#[diesel(table_name = crate::schema::user_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserSession {
    pub id: Uuid,
    pub access_token: String,
}
