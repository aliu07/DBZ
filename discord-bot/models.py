from pydantic import BaseModel
from datetime import datetime

class Practice(BaseModel):
    practice_id: str
    start_time: datetime
    end_time: datetime
