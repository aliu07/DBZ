from typing import Final
import os
from dotenv import load_dotenv
from discord import Intents, Client

# LOAD TOKEN
load_dotenv()
TOKEN: Final[str] = os.getenv('DISCORD_TOKEN') or ''
if not TOKEN:
    raise ValueError("DISCORD_TOKEN environment variable is not set")

# BOT SETUP
intents: Intents = Intents.default()
intents.members = True
intents.presences = True
intents.messages = True
intents.message_content = True  # NOQA
client: Client = Client(intents=intents)


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


# MAIN ENTRY POINT
def main() -> None:
    client.run(token=TOKEN)


if __name__ == '__main__':
    main()
