from datetime import datetime

def lpad(s: any, ch: str, target_length: int):
	return ch * (target_length - len(str(s))) + str(s)

def format_timestamp(dt: datetime):
	formatted = f"{lpad(dt.hour, '0', 2)}:{lpad(dt.minute, '0', 2)}:{lpad(dt.second, '0', 2)} "

	now = datetime.now()
	if now.day == dt.day and now.month == dt.month and now.year == dt.year:
		formatted += "today"
	elif now.day == dt.day + 1 and now.month == dt.month and now.year == dt.year:
		formatted += "yesterday"
	else:
		formatted += f"{lpad(dt.day, '0', 2)}/{lpad(dt.month, '0', 2)}/{dt.year}"

	return formatted