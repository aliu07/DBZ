from pydantic import BaseModel, Field
from datetime import datetime
from typing import Optional, List

class Practice(BaseModel):
    practice_id: str
    start_time: datetime
    end_time: datetime

class WaitlistedMessageRequest(BaseModel):
    practice: Practice
    discord_id: int
