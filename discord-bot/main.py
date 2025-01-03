from json import JSONDecodeError
import os
import asyncio
import uvicorn
import re
from typing import Final
from api_server import init_api, app
import aiohttp

from dotenv import load_dotenv
from discord import Intents, Client, Message, Embed, Color, DMChannel, Interaction, RawReactionActionEvent
from discord.ext import commands

# LOAD TOKEN
load_dotenv()
TOKEN: Final[str] = os.getenv('DISCORD_TOKEN') or ''
CHANNEL_ID: Final[int] = int(os.getenv('CHANNEL_ID') or 0)
if not TOKEN:
    raise ValueError("DISCORD_TOKEN environment variable is not set")
if not CHANNEL_ID:
    raise ValueError("CHANNEL_ID environment variable is not set")
URL: Final[str] = os.getenv('BACKEND_API_URL') or 'http://backend:8000'  # Base URL

# BOT SETUP
intents: Intents = Intents.default()
intents.members = True
intents.presences = True
intents.messages = True
intents.message_content = True
intents.guilds = True
intents.dm_messages = True
intents.dm_typing = True     # Add this
intents.dm_reactions = True  # Add this
intents.guild_messages = True # Add this

client = commands.Bot(
    command_prefix='!',
    intents=intents,
    help_command=None
)

client.channel_id = CHANNEL_ID

waiting_for_email = {}

def is_valid_email(email: str) -> bool:
    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
    return bool(re.match(pattern, email))

async def register_user(user_id: str, email: str):
    async with aiohttp.ClientSession() as session:
        try:
            payload = {
                "email": email,
                "discord_id": user_id
            }
            headers = {
                "Content-Type": "application/json"
            }
            full_url = f"{URL}/register"
            print(f"Sending request to: {full_url}")  # Debug print
            print(f"Payload: {payload}")
            async with session.post(full_url, json=payload, headers=headers) as response:
                if response.status == 200:
                    return True, "Registration successful!"
                else:
                    response_text = await response.text()
                    return False, response_text
        except aiohttp.ClientError as e:
            return False, f"Failed to connect to backend: {str(e)}"
        except Exception as e:
            return False, f"Unexpected error: {str(e)}"


async def sign_up_for_practice(practice_id: str, user_id: str):
    async with aiohttp.ClientSession() as session:
        try:
            payload = {
                "practice_id": practice_id,
                "discord_id": user_id
            }
            headers = {
                "Content-Type": "application/json"
            }
            full_url = f"{URL}/practice/signup"

            print(f"Sending request to: {full_url}")  # Debug print
            print(f"Payload: {payload}")

            async with session.post(full_url, json=payload, headers=headers) as response:
                text_response = await response.text()
                print(f"Response status: {response.status}")
                print(f"Response text: {text_response}")
                return response.status == 200, text_response.strip('"'), False
        except aiohttp.ClientError as e:
            return False, f"Failed to connect to backend: {str(e)}", False
        except Exception as e:
            return False, f"Unexpected error: {str(e)}", False

async def unregister_from_practice(practice_id: str, user_id: str):
    async with aiohttp.ClientSession() as session:
        try:
            payload = {
                "practice_id": practice_id,
                "discord_id": user_id
            }
            headers = {
                "Content-Type": "application/json"
            }
            full_url = f"{URL}/practice/unregister"

            print(f"Sending unregister request to: {full_url}")  # Debug print
            print(f"Payload: {payload}")

            async with session.delete(full_url, json=payload, headers=headers) as response:
                text_response = await response.text()
                print(f"Response status: {response.status}")
                print(f"Response text: {text_response}")
                return response.status == 200, text_response.strip('"')
        except aiohttp.ClientError as e:
            return False, f"Failed to connect to backend: {str(e)}"
        except Exception as e:
            return False, f"Unexpected error: {str(e)}"



# HANDLING BOT STARTUP
@client.event
async def on_ready() -> None:
    await client.tree.sync()
    print(f'{client.user} is now running!')





# WHEN MEMBER JOINS
@client.event
async def on_member_join(member) -> None:
    try:
        welcome_embed = Embed(
            title="Welcome to the Server!",
            description="Please provide your email address for registration.",
            color=Color.red()
        )
        welcome_embed.add_field(
            name="Instructions",
            value="Please reply to this message with your email address.",
            inline=False
        )

        await member.send(embed=welcome_embed)
        waiting_for_email[member.id] = True

    except Exception as e:
        print(f"Couldn't send direct message to {member.name}: {e}")





# Handle DM messages for email collection
@client.event
async def on_message(message: Message) -> None:
    # Ignore messages from the bot itself
    if message.author == client.user:
        return

    # Check if message is a DM and user is waiting for email verification
    if isinstance(message.channel, DMChannel) and message.author.id in waiting_for_email:
        email = message.content.strip()

        if not is_valid_email(email):
            await message.channel.send("Invalid email format. Please provide a valid email address.")
            return

        # Send to backend
        success, response_message = await register_user(
            str(message.author.id),
            email
        )

        if success:
            await message.channel.send(
                embed=Embed(
                    title="Registration Complete",
                    description=response_message,
                    color=Color.green()
                )
            )
            # Remove user from waiting list
            del waiting_for_email[message.author.id]
        else:
            await message.channel.send(
                embed=Embed(
                    title="Registration Failed",
                    description=f"Sorry, there was an error: {response_message}",
                    color=Color.red()
                )
            )
        return

    # Process commands as normal
    await client.process_commands(message)




# HANDLE REACTIONS
@client.event
async def on_raw_reaction_add(payload: RawReactionActionEvent):
    if payload.emoji.name != "✅":
        return

    channel = client.get_channel(payload.channel_id)

    if not channel:
        return

    message = await channel.fetch_message(payload.message_id)

    if not message:
        return

    if message.author != client.user or not message.embeds:
        return

    user_id = str(payload.user_id)
    practice_id = None

    for field in message.embeds[0].fields:
        if field.name == "practice_id":
            practice_id = field.value
            break

    if practice_id:
        print(f"User {user_id} reacted to practice {practice_id}")
        success, message, on_waitlist = await(sign_up_for_practice(practice_id, user_id))
        user = await client.fetch_user(int(user_id))

        if success or on_waitlist:
            await user.send(f"{message}")
        else:
            await user.send(f"An error occured... {message}")

@client.event
async def on_raw_reaction_remove(payload: RawReactionActionEvent):
    if payload.emoji.name != "✅":
        return

    channel = client.get_channel(payload.channel_id)

    if not channel:
        return

    message = await channel.fetch_message(payload.message_id)

    if not message:
        return

    if message.author != client.user or not message.embeds:
        return

    user_id = str(payload.user_id)
    practice_id = None

    for field in message.embeds[0].fields:
        if field.name == "practice_id":
            practice_id = field.value
            break

    if practice_id:
        print(f"User {user_id} removed reaction from practice {practice_id}")
        success, message = await unregister_from_practice(practice_id, user_id)
        user = await client.fetch_user(int(user_id))

        if success:
            await user.send(
                embed=Embed(
                    title="Practice Registration Cancelled",
                    description=message,
                    color=Color.orange()
                )
            )
        else:
            await user.send(
                embed=Embed(
                    title="Error Cancelling Registration",
                    description=f"An error occurred: {message}",
                    color=Color.red()
                )
            )

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





# COMMANDS

@client.tree.command(name="schedule", description="Sends the semester's schedule")
async def schedule(interaction: Interaction):
    await interaction.response.send_message(f'Here is the schedule {interaction.user.name}!')



@client.tree.command(name="lineup", description="Sends the next practice lineup")
async def lineup(interaction: Interaction):
    await interaction.response.send_message(f'Here is the lineup {interaction.user.name}!')



@client.tree.command(name="fun_fact", description="Gives a fun fact!")
async def fun_fact(interaction: Interaction):
    await interaction.response.send_message(f'Hey {interaction.user.name}! \n Did you know that your VP Finance, Alexander has not been in ONE DBZ Tiktok??')



@client.tree.command(name="help", description="Shows all available commands")
async def help(interaction: Interaction):
    help_embed = Embed(
        title="Bot Commands",
        description="Here are all available commands:",
        color=Color.red()
    )

    for command in client.tree.walk_commands():
        help_embed.add_field(
            name=f"!{command.name}",
            value=command.description or "No description available",
            inline=False
        )

    await interaction.response.send_message(embed=help_embed)





# MAIN ENTRY POINT
async def main() -> None:
    init_api(client)
    await asyncio.gather(run_discord_bot(), run_api_server())





if __name__ == '__main__':
    asyncio.run(main())
