-- Your SQL goes here
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY,
    access_token VARCHAR(10240) NOT NULL
);
