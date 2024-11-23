import discord
from discord.ext import commands
import os
from dotenv import load_dotenv

TOKEN = os.getenv('DISCORD_TOKEN')

client = commands.Bot(command_prefix = "!")

@client.event
async def on_ready():
    print("Ready to go")
    print("===========")

@client.command()
async def hello(ctx):
    await ctx.send("Hello, DBZ Bot ready")

client.run(TOKEN)
