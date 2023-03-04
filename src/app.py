from flask import Flask, Markup, render_template, send_from_directory, request, session, flash, redirect
from flask_session import Session
import database




# creating app
app = Flask(__name__, template_folder="../templates", static_folder="../static")
with open("../secret_key", "r") as f:
	app.secret_key = f.read()

# session configuration
app.config["SESSION_PERMANENT"] = False
app.config["SESSION_TYPE"] = "filesystem"
Session(app)

# static files
@app.route("/static/<path:path>")
def send_static(path):
	return send_from_directory("static", path)



# posts
@app.route("/")
def feed():
	return render_template("feed.html", userdata=session, posts=database.get_posts(-1))

@app.route("/tweet/<post>")
def get_post(post):
	print(database.get_post(post))
	return f"<h1>{post}</h1>"

@app.route("/post/", methods=["GET", "POST"])
def create_post():
	if request.method == "GET":
		if not session.get("login"):
			return redirect("/")
		return render_template("create_post.html", userdata=session)

	login = session["login"]
	post_body = request.form["body"]

	if not post_body:
		flash("Please enter something")
	else:
		database.post(login, post_body)
		return redirect("/")

	return render_template("create_post.html", userdata=session)



# account
@app.route("/login/", methods=["GET", "POST"])
def login():
	if request.method == "GET":
		if session.get("login"):
			return redirect("/")
		return render_template("login.html")
	
	login = request.form["login"]
	password = request.form["password"]

	if not login:
		flash("Please enter a login")
	elif not password:
		flash("Please enter a password")
	else:
		uuid = database.log_in(login, password)
		if not uuid:
			flash("Incorrect login or password")
		else:
			session["login"] = login
			return redirect("/")

	return render_template("login.html", userdata=session)

@app.route("/register/", methods=["GET", "POST"])
def register():
	if request.method == "GET":
		if session.get("login"):
			return redirect("/")
		return render_template("register.html")

	login = request.form["login"]
	password = request.form["password"]

	if not login:
		flash("Please enter a login")
	elif not password:
		flash("Please enter a password")
	else:
		uuid = database.register(login, password)
		if not uuid:
			flash("Account with this login already exists")
		else:
			session["login"] = login
			return redirect("/")

	return render_template("register.html")

@app.route("/signout/")
def signout():
	session.pop("login", None)
	session.pop("password", None)

	return redirect("/")



# launching app
if __name__ == "__main__":
	app.run(debug=True, host="0.0.0.0", port=7777)
