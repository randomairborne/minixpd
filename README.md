# miniXPD

miniXPD is a self-hostable discord bot for leveling.

## How to host

MiniXPD is a single docker container. It requires two environment variables: a PostgreSQL database url and a discord token.

You can run it like so:

```bash
docker run -e DISCORD_TOKEN=a.b.c -e DATABASE_URL=postgres://username:password@address/database
```

Note that the address must be absolute, as docker does weird things with localhost networking.
