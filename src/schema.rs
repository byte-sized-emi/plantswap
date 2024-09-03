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
    images (file_key) {
        file_key -> Uuid,
        uploaded_by_user -> Nullable<Uuid>,
        upload_date -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ListingType;

    listings (id) {
        id -> Int4,
        #[max_length = 120]
        title -> Varchar,
        #[max_length = 1023]
        description -> Varchar,
        insertion_date -> Timestamp,
        author -> Uuid,
        listing_type -> ListingType,
        thumbnail -> Nullable<Uuid>,
        tradeable -> Bool,
        identified_plant -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
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

diesel::joinable!(listings -> images (thumbnail));
diesel::joinable!(listings -> plants (identified_plant));

diesel::allow_tables_to_appear_in_same_query!(
    images,
    listings,
    plants,
);
