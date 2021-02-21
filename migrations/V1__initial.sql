CREATE TABLE users
(
    userid uuid PRIMARY KEY,
    username text UNIQUE NOT NULL,
    realname text,
    committername text,
    emails text[] NOT NULL,
    password_hash text NOT NULL,
    totp_secret bytea
);

CREATE TABLE repositories
(
    repo_id uuid PRIMARY KEY,
    owner_id uuid NOT NULL,
    vcs text NOT NULL,
    repo_name text NOT NULL,
    primary_branch text NOT NULL,
    repo_description text
);
