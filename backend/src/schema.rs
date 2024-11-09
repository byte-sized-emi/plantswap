// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "listing_type"))]
    pub struct ListingType;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "plant_location"))]
    pub struct PlantLocation;
}

diesel::table! {
    use diesel::sql_types::*;
    use postgis_diesel::sql_types::*;

    images (file_key) {
        file_key -> Uuid,
        uploaded_by_user -> Nullable<Uuid>,
        upload_date -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use postgis_diesel::sql_types::*;
    use super::sql_types::ListingType;

    listings (id) {
        id -> Uuid,
        #[max_length = 120]
        title -> Varchar,
        #[max_length = 1023]
        description -> Varchar,
        insertion_date -> Timestamp,
        author -> Uuid,
        listing_type -> ListingType,
        thumbnail -> Uuid,
        tradeable -> Bool,
        identified_plant -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use postgis_diesel::sql_types::*;
    use super::sql_types::PlantLocation;

    plants (id) {
        id -> Int4,
        #[max_length = 63]
        human_name -> Varchar,
        #[max_length = 127]
        species -> Varchar,
        location -> Nullable<PlantLocation>,
        produces_fruit -> Nullable<Bool>,
        #[max_length = 1023]
        description -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use postgis_diesel::sql_types::*;

    spatial_ref_sys (srid) {
        srid -> Int4,
        #[max_length = 256]
        auth_name -> Nullable<Varchar>,
        auth_srid -> Nullable<Int4>,
        #[max_length = 2048]
        srtext -> Nullable<Varchar>,
        #[max_length = 2048]
        proj4text -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use postgis_diesel::sql_types::*;

    user_sessions (id) {
        id -> Uuid,
        #[max_length = 10240]
        access_token -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use postgis_diesel::sql_types::*;

    users (id) {
        id -> Uuid,
        location -> Nullable<Geography>,
    }
}

diesel::joinable!(listings -> images (thumbnail));
diesel::joinable!(listings -> plants (identified_plant));
diesel::joinable!(listings -> users (author));

diesel::allow_tables_to_appear_in_same_query!(
    images,
    listings,
    plants,
    spatial_ref_sys,
    user_sessions,
    users,
);
