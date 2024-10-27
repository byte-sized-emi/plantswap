-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS listings;
DROP TABLE IF EXISTS images;
DROP TABLE IF EXISTS plants;
DROP TABLE IF EXISTS users;
DROP TYPE IF EXISTS listing_type;
DROP TYPE IF EXISTS plant_location;

DROP EXTENSION IF EXISTS postgis;
