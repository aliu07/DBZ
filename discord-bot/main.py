import discord
from discord.etx import commands

client = commands.Bot(command_prefix = "!")

@client.event
async def on_ready():
    print("Ready to go")
    print("===========")

@client.command()
async def hello(ctx):
    await ctx.send("Hello, DBZ Bot ready")

client.run()
