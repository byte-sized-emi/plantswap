-- for geospatial data
CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TYPE listing_type AS ENUM ('selling', 'buying');
CREATE TYPE plant_location AS ENUM ('outdoor', 'indoor');

CREATE TABLE users (
    id UUID not null PRIMARY KEY,
    location geography(POINT,4326)
);

CREATE TABLE plants (
    id SERIAL PRIMARY KEY,
    human_name VARCHAR(63) NOT NULL,
    species VARCHAR(127) NOT NULL,
    location plant_location,
    produces_fruit BOOLEAN,
    description VARCHAR(1023) NOT NULL
);

CREATE TABLE images (
    file_key UUID PRIMARY KEY,
    uploaded_by_user UUID,
    upload_date TIMESTAMP NOT NULL DEFAULT NOW()
);

-- images are stored in a separate table and only reference this
CREATE TABLE listings (
    id uuid PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    title VARCHAR(120) NOT NULL,
    description VARCHAR(1023) NOT NULL,
    insertion_date TIMESTAMP NOT NULL DEFAULT NOW(),
    author UUID NOT NULL REFERENCES users,
    listing_type listing_type NOT NULL,
    thumbnail UUID NOT NULL REFERENCES images,
    -- whether or not the author would accept a trade
    tradeable BOOLEAN NOT NULL DEFAULT false,
    identified_plant int REFERENCES plants
);
