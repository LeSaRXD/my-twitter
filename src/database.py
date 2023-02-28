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
def get_posts(post_amount: int = 10) -> str:
	if post_amount < 0:
		cur.execute("SELECT * FROM posts ORDER BY time DESC")
	else:
		cur.execute("SELECT * FROM posts ORDER BY time DESC LIMIT %s", (post_amount, ))

	posts = cur.fetchall()

	return [{"username": get_username_by_uuid(post["id"]), "body": post["body"], "timestamp": format_timestamp(post["time"])} for post in posts]

# TODO: update post function
def post(uuid: str, body: str) -> None:
	if uuid == None:
		return
	
	# check if uuid exists
	cur.execute("SELECT (id) FROM users LIMIT 1")
	if cur.fetchone() == None:
		return

	cur.execute("INSERT INTO posts (id, body) VALUES(%s, %s)", (uuid, body))
	conn.commit()



# accounts
def get_username_by_uuid(uuid: str) -> str:
	cur.execute("SELECT (username) FROM users WHERE id=%s LIMIT 1", (uuid, ))
	return cur.fetchone()["username"]

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
