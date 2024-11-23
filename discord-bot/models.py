from pydantic import BaseModel
from datetime import datetime

class Practice(BaseModel):
    practice_id: str
    datetime: datetime
