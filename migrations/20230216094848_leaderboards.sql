-- Add migration script here
CREATE TABLE leaderboards (
    invoker BIGINT NOT NULL,
    message BIGINT NOT NULL,
    page BIGINT NOT NULL,
    token VARCHAR(512) NOT NULL
)