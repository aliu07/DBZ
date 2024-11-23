import os
import asyncio
import uvicorn

from typing import Final
from dotenv import load_dotenv
from discord import Intents, Client
from api_server import init_api, app


# LOAD TOKEN
load_dotenv()
TOKEN: Final[str] = os.getenv('DISCORD_TOKEN') or ''
CHANNEL_ID: Final[int] = int(os.getenv('CHANNEL_ID') or 0)
if not TOKEN:
    raise ValueError("DISCORD_TOKEN environment variable is not set")
if not CHANNEL_ID:
    raise ValueError("CHANNEL_ID environment variable is not set")

# BOT SETUP
intents: Intents = Intents.default()
intents.members = True
intents.presences = True
intents.messages = True
intents.message_content = True  # NOQA
client: Client = Client(intents=intents)
client.channel_id = CHANNEL_ID


# HANDLING BOT STARTUP
@client.event
async def on_ready() -> None:
    print(f'{client.user} is now running!')


# WHEN MEMBER JOINS
@client.event
async def on_member_join(member) -> None:
    try:
        await member.send(f"Welcome to the server, {member.name}!")
    except Exception as e:
        print(f"Couldn't send direct message to {member.name}: {e}")

async def run_discord_bot():
    await client.start(token=TOKEN)

async def run_api_server():
    config = uvicorn.Config(
        app,
        host="0.0.0.0",
        port=3001,
        log_level="info"
    )
    server = uvicorn.Server(config)
    await server.serve()

# MAIN ENTRY POINT
async def main() -> None:
    init_api(client)
    await asyncio.gather(run_discord_bot(), run_api_server())


if __name__ == '__main__':
    asyncio.run(main())
