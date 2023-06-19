# minixpd

# !!!!! minixpd IS UNMAINTAINED !!!!!

the xpd-lite container and crate on [randomairborne/experienced](/randomairborne/experienced/) has replaced its functionality.

minixpd (stylized in all lowercase) is a simple Discord bot forked from [randomairborne/experienced](/randomairborne/experienced/)
with the purpose of being an easy-to-host bot for leveling.

The easisest way to self-host minixpd is with Docker on Linux. Thus, this is how this tutorial will set up minixpd.

If you'd rather just have a hosted bot, that's fine! [Click here to invite it.](https://discord.com/api/oauth2/authorize?client_id=1035970092284002384&permissions=0&scope=bot%20applications.commands)

## Preparing your server

To run minixpd, you need [docker](https://docs.docker.com/engine/install/) with the [compose extension](https://docs.docker.com/compose/install/linux/).

## Creating a Discord application

Now we need to create a Discord application. Go to [https://discord.com/developers/applications](https://discord.com/developers/applications) and click the `New Application` button in the top right corner.
Give it a nice name, then click continue. Now we need to create a .env file, which should look like this:

```dotenv
DISCORD_TOKEN=
```

Go to the `Bot` tab and click `Add Bot`. This will also show you a `Reset Token` button. Clicking this should reveal and copy your bot token,
which should then be filled into the `DISCORD_TOKEN`. Then, customize your bot to your heart's content. No gateway intents are needed. While you are legally within your rights to do so, please do not self-host public instances of minixpd. It's not designed for that.

## Starting the bot

Now we need to get the docker-compose file for minixpd. You can download it with cURL.

```bash
curl -o compose.yaml https://raw.githubusercontent.com/randomairborne/minixpd/main/compose.yaml
```

Put this in the same folder as your `.env` file. Then, you can run the bot with

```bash
docker compose up -d
```

You can stop the bot with

```bash
docker compose down
```

Every once in a while, update the bot with

```bash
docker compose pull && docker compose down && docker compose up -d
```
