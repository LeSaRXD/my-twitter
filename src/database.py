from passwords import *
from timestamps import *
import psycopg2
import psycopg2.extras

db_user = "lesar"
db_name = "twitter"

connection = psycopg2.connect(database=db_name, user=db_user)
cur = connection.cursor(cursor_factory=psycopg2.extras.DictCursor)

def get_username_by_uuid(uuid: str) -> str:
	cur.execute("SELECT (username) FROM users WHERE id=%s LIMIT 1", (uuid, ))
	return cur.fetchone()["username"]

def get_posts(post_amount: int = 10) -> str:
	if post_amount < 0:
		cur.execute("SELECT * FROM posts ORDER BY time DESC")
	else:
		cur.execute("SELECT * FROM posts ORDER BY time DESC LIMIT %s", (post_amount, ))

	posts = cur.fetchall()

	return [{"username": get_username_by_uuid(post["id"]), "body": post["body"], "timestamp": format_timestamp(post["time"])} for post in posts]

def post(uuid: str, body: str) -> None:
	if uuid == None:
		return
	
	# check if uuid exists
	cur.execute("SELECT (id) FROM users LIMIT 1")
	if cur.fetchone() == None:
		return

	cur.execute("INSERT INTO posts (id, body) VALUES(%s, %s)", (uuid, body))
	connection.commit()

def register(login: str, password: str) -> None:
	cur.execute("INSERT INTO users (username, password_hash) VALUES (%s, %s)", (login, encode_pw(password)))
	cur.commit()

def login(login: str, password: str) -> str:
	cur.execute("SELECT id, password_hash FROM users WHERE username=%s", (login, ))
	account = cur.fetchone()
	
	if account == None:
		return False

	if not validate_pw(password, account["password_hash"]):
		return False
	
	return account["id"]
