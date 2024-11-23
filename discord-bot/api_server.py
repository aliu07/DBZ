from fastapi import FastAPI, HTTPException, status
from models import Practice
from discord import Client


app = FastAPI()
discord_client: Client | None = None

def init_api(client: Client):
    global discord_client
    discord_client = client


@app.post('/practice', status_code=status.HTTP_201_CREATED)
async def create_practice(practice: Practice):
    if not discord_client:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Discord client not initialized..."
        )

    channel_id = int(discord_client.channel_id)
    channel = discord_client.get_channel(channel_id)

    if channel:
        await channel.send(f"Practice on {practice.start_time.strftime('%Y-%m-%d %H:%M')} is open!")
        return {
            "status": "success",
            "message": "Practice notification sent"
        }

    raise HTTPException(
        status_code=status.HTTP_404_NOT_FOUND,
        detail="Channel ID not found..."
    )
