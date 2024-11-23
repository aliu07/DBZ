# discord-bot/Dockerfile

# Use the official Python image
FROM python:3.11-slim

# Set environment variables
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# Set the working directory
WORKDIR /app

# Install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy the bot source code
COPY bot.py .

# Optionally, copy other necessary files or directories
# COPY utils.py ./utils.py

# Define the command to run the bot
CMD ["python", "bot.py"]
