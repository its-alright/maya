CREATE SCHEMA IF NOT EXISTS api;


CREATE TABLE api.orders (
    uid uuid NOT NULL DEFAULT uuidv7(),
    user_id INTEGER NOT NULL,
    ogrn varchar(13) NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    modified_at timestamptz NOT NULL DEFAULT NOW(),
    version INTEGER NOT NULL,
    CONSTRAINT orders_pkey PRIMARY KEY (uid)
);

CREATE INDEX orders_created_at_idx ON ONLY api.orders USING btree (created_at);
CREATE INDEX orders_user_id_uid_idx ON ONLY api.orders USING btree (user_id, uid);