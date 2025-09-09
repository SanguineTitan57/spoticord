-- Add migration script here
-- Create user table
CREATE TABLE "user" (
    id VARCHAR PRIMARY KEY,
    device_name VARCHAR(32) NOT NULL
);

-- Create account table
CREATE TABLE account (
    user_id VARCHAR NOT NULL,
    username VARCHAR(64) NOT NULL,
    access_token VARCHAR(1024) NOT NULL,
    refresh_token VARCHAR(1024) NOT NULL,
    session_token VARCHAR(1024),
    expires TIMESTAMP NOT NULL,
    last_updated TIMESTAMP NOT NULL,
    PRIMARY KEY (user_id),
    CONSTRAINT fk_account_user FOREIGN KEY (user_id) REFERENCES "user"(id) ON DELETE CASCADE
);

-- Create link_request table
CREATE TABLE link_request (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires TIMESTAMP NOT NULL,
    CONSTRAINT fk_link_request_user FOREIGN KEY (user_id) REFERENCES "user"(id) ON DELETE CASCADE
);

