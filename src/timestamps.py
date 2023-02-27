from datetime import datetime

def lpad(s: any, ch: str, target_length: int):
	return ch * (target_length - len(str(s))) + str(s)

def format_timestamp(dt: datetime):
    return f"{lpad(dt.hour, '0', 2)}:{lpad(dt.minute, '0', 2)}:{lpad(dt.second, '0', 2)} {lpad(dt.day, '0', 2)}/{lpad(dt.month, '0', 2)}/{dt.year}"
