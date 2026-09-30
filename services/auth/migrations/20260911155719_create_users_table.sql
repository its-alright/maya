CREATE SCHEMA IF NOT EXISTS auth;

CREATE TABLE IF NOT EXISTS auth.users
(
    id              uuid PRIMARY KEY NOT NULL DEFAULT uuidv7(),
    created_at      TIMESTAMPTZ      NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ      NOT NULL DEFAULT now(),
    email           TEXT UNIQUE      NOT NULL,
    display_name    TEXT             NOT NULL,
    hashed_password TEXT             NOT NULL,
    salt            TEXT             NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_email ON auth.users (email);