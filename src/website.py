from flask import Flask, Markup, render_template, send_from_directory, request, session, flash, redirect, make_response
from flask_session import Session
import database



# creating app
app = Flask(__name__, template_folder="../templates", static_folder="../static")
app.secret_key = "dev"

# session configuration
app.config["SESSION_PERMANENT"] = False
app.config["SESSION_TYPE"] = "filesystem"
Session(app)

# static files
@app.route("/static/<path:path>")
def send_static(path):
	return send_from_directory("static", path)

# main page
@app.route("/")
def feed():
	return render_template("feed.html", userdata=session, tweets=database.get_posts(10))

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
			resp = make_response(redirect("/"))
			return resp

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
			resp = make_response(redirect("/"))
			return resp

	return render_template("register.html")

@app.route("/signout/")
def signout():
	session.pop("login", None)
	session.pop("password", None)

	resp = make_response(redirect("/"))
	return resp



# launching app
if __name__ == "__main__":
	app.run(debug=True)
