import passwords
from timestamps import *
import psycopg2
import psycopg2.extras
import psycopg2.errors



# accessing the database
db_user = "lesar"
db_name = "twitter"

conn = psycopg2.connect(database=db_name, user=db_user)
cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)



# posts
def db_post_to_dict(post: psycopg2.extras.DictRow) -> dict:
	return {"id": post["id"], "username": get_username_by_uuid(post["poster_id"]), "body": post["body"], "timestamp": format_timestamp(post["time"]), "likes": post["likes"]}

def get_post(post_id: str) -> dict:
	cur.execute("SELECT * FROM posts WHERE id=%s", (post_id, ))
	post = cur.fetchone()
	if not post:
		return False

	return db_post_to_dict(post)

def get_posts(post_amount: int = 10) -> list:
	if post_amount < 0:
		cur.execute("SELECT * FROM posts ORDER BY time DESC")
	else:
		cur.execute("SELECT * FROM posts ORDER BY time DESC LIMIT %s", (post_amount, ))

	posts = cur.fetchall()

	return [db_post_to_dict(post) for post in posts]

def post(login: str, body: str) -> bool:
	print("POSTING")

	uuid = get_uuid_by_username(login)
	if not uuid:
		print("NO uuid")
		return False

	try:
		cur.execute("INSERT INTO posts (poster_id, body) VALUES(%s, %s)", (uuid, body))
		conn.commit()
	except:
		conn.rollback()
		return False

	return True



# accounts
def get_username_by_uuid(uuid: str) -> str:
	cur.execute("SELECT (username) FROM users WHERE id=%s LIMIT 1", (uuid, ))
	return cur.fetchone()["username"]

def get_uuid_by_username(username: str) -> str:
	cur.execute("SELECT (id) FROM users WHERE username=%s LIMIT 1", (username, ))
	return cur.fetchone()["id"]

def log_in(login: str, password: str) -> str:
	cur.execute("SELECT id, password_hash FROM users WHERE username=%s", (login, ))
	account = cur.fetchone()
	
	if account == None:
		return False

	if not passwords.validate_pw(password, account["password_hash"]):
		return False

	update_password(login, password)

	return account["id"]

def register(login: str, password: str):
	print("AAAAAAAAAA:", password)
	try:
		cur.execute("INSERT INTO users (username, password_hash) VALUES (%s, %s)", (login, passwords.encode_pw(password)[0]))
		conn.commit()
	except psycopg2.errors.UniqueViolation:
		conn.rollback()
		return False

	return log_in(login, password)

def update_password(login: str, new_password: str) -> None:
	cur.execute("UPDATE users SET password_hash=%s WHERE username=%s", (passwords.encode_pw(new_password)[0], login))
	conn.commit()
