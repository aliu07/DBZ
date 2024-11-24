from fastapi import FastAPI, HTTPException, status
from models import Practice
from discord import Client, Embed, Color
from datetime import *


app = FastAPI()
discord_client: Client | None = None
sign_up_limit = 34
waitlist_limit = 12

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
        # Format dates and times
        practice_date = practice.start_time.strftime('%A, %B %d, %Y')
        start_time = practice.start_time.strftime('%I:%M %p')
        end_time = practice.end_time.strftime('%I:%M %p')

        # Calculate duration
        duration = practice.end_time - practice.start_time
        duration_hours = duration.total_seconds() / 3600

        practice_embed = Embed(
            title="🏃 New Practice Session",
            description="A new practice session has been scheduled!",
            color=Color.red(),
        )

        practice_embed.add_field(
            name="📅 Date",
            value=practice_date,
            inline=True
        )
        practice_embed.add_field(
            name="⏰ Time",
            value=f"{start_time} - {end_time}\n({duration_hours:.1f} hours)",
            inline=True
        )

        practice_embed.add_field(
            name="👥 Capacity",
            value=f"```34 spots\nWaitlist: 12 spots```",
            inline=False
        )

        practice_embed.add_field(
            name="📝 How to Sign Up",
            value="• React with ✅ to join practice\n"
                    "• Remove your reaction to cancel",
            inline=False
        )

        message = await channel.send(embed=practice_embed)
        await message.add_reaction("✅")  # Participate

        return {
            "status": "success",
            "message": "Practice notification sent",
            "practice_id": practice.practice_id,
            "channel_id": channel_id
        }

    raise HTTPException(
        status_code=status.HTTP_404_NOT_FOUND,
        detail="Channel ID not found..."
    )

@app.post('/waitlisted-msg', status_code=status.HTTP_201_CREATED)
async def send_msg_to_waitlisted_user(practice: Practice, discord_id):
    user = discord_client.get_user(discord_id)
    if user:
        message = f"Hey {user.name}, you have 5 MINUTES to react to claim for practice {practice.practice_id} starting at {practice.start_time} and ending at {practice.end_time}!"
        await user.send(message)
        return {
            "status": "success",
            "message": "Message sent to user"
        }

    raise HTTPException(
        status_code=status.HTTP_404_NOT_FOUND,
        detail="User not found..."
    )

