-- Add migration script here
CREATE TABLE levels (
    id BIGINT NOT NULL,
    xp BIGINT NOT NULL,
    guild BIGINT NOT NULL,
    PRIMARY KEY (id, guild)
);

CREATE TABLE card_toy (
    toy VARCHAR(64) NOT NULL,
    id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    PRIMARY KEY (id, guild_id)
);

CREATE TABLE role_rewards (
    id BIGINT NOT NULL,
    requirement BIGINT NOT NULL,
    guild BIGINT NOT NULL,
    UNIQUE (guild, id),
    UNIQUE (guild, requirement)
);