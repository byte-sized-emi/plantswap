
CREATE TYPE listing_type AS ENUM ('selling', 'buying');
CREATE TYPE plant_location AS ENUM ('outdoor', 'indoor');

CREATE TABLE plants (
    id SERIAL PRIMARY KEY,
    human_name VARCHAR(63) NOT NULL,
    species VARCHAR(127) NOT NULL,
    location plant_location,
    produces_fruit BOOLEAN,
    description VARCHAR(1023) NOT NULL DEFAULT ''
);

CREATE TABLE images (
    file_key UUID PRIMARY KEY,
    uploaded_by_user UUID,
    upload_date TIMESTAMP NOT NULL DEFAULT NOW()
);

-- images are stored in a separate table and only reference this
CREATE TABLE listings (
    id SERIAL PRIMARY KEY,
    title VARCHAR(120) NOT NULL,
    description VARCHAR(1023) NOT NULL,
    insertion_date TIMESTAMP NOT NULL DEFAULT NOW(),
    author UUID NOT NULL,
    listing_type listing_type NOT NULL,
    thumbnail UUID REFERENCES images,
    -- whether or not the author would accept a trade
    tradeable BOOLEAN NOT NULL DEFAULT false,
    identified_plant int REFERENCES plants
);
