from datetime import datetime
from passwords import *
from timestamps import *
import psycopg2

db_user = "lesar"
db_name = "twitter"

connection = psycopg2.connect(database=db_name, user=db_user)
cur = connection.cursor()

user = None
user_uuid = None

def feed(post_amount: int = 10) -> str:
	cur.execute("SELECT * FROM posts ORDER BY post_time DESC LIMIT %s", (post_amount, ))
	posts = cur.fetchall()
	if len(posts) == 0:
		return "No recent posts"
	
	posts_str = ""

	for post in posts:
		cur.execute("SELECT * FROM users WHERE client_id=%s", (post[0], ))
		poster = cur.fetchone()[1]
		posts_str += f"\n\n\n@{poster} posted at {datetime_str(post[2])}:\n{post[1]}"

	return posts_str

def login():
	global user
	global user_uuid

	login = input("Enter login: ")
	if len(login) == 0:
		print("Cannot log in with blank login")
		return
	
	password = input("Enter password: ")
	if len(password) == 0:
		print("Cannot log in with blank password")
		return
	
	cur.execute("SELECT * FROM users WHERE username=%s", (login, ))
	fetched_user = cur.fetchone()

	if fetched_user == None:
		print(f"No user with login {login} found")
		return

	hashed_pw = fetched_user[2]
	if validate_pw(password, hashed_pw):
		print("Logged in!")
		user = login
		user_uuid = fetched_user[0]
	else:
		print("Incorrect password!")

def register():
	login = input("Enter new login: ")
	password = input("Enter new password: ")
	cur.execute("INSERT INTO users (username, password_sha256) VALUES(%s, %s)", (login, encode_pw(password)[0]))
	connection.commit()
	print(f"Created account {login}")

def post():
	if user == None:
		print("You need to login to post!")
		return

	post_body = input("Enter post (max 512 characters):\n")
	cur.execute("INSERT INTO posts (poster_id, post_body, post_time) VALUES(%s, %s, %s)", (user_uuid, post_body, datetime.now()))
	connection.commit()
	print("Posted!")



def main():
	print("Welcome to my test blog")

	choice = "1"
	while choice in ["1", "2", "3", "4"]:
		print("\n\n")
		print("What would you like to do?", "1 - View recent posts", "2 - Login", "3 - Register", "4 - Post", "Other - exit", sep="\n")

		choice = input(">>> ")
		if choice == "1":
			print(feed())
		elif choice == "2":
			login()
		elif choice == "3":
			register()
		elif choice == "4":
			post()

if __name__ == "__main__":
	main()