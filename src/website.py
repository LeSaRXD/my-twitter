from flask import Flask, Markup, render_template, send_from_directory, request, session, flash, redirect, make_response
from flask_session import Session
import database



app = Flask(__name__, template_folder="../templates", static_folder="../static")
app.secret_key = "dev"
app.config["SESSION_PERMANENT"] = False
app.config["SESSION_TYPE"] = "filesystem"
Session(app)

@app.route("/static/<path:path>")
def send_static(path):
	return send_from_directory("static", path)

@app.route("/")
def feed():
	return render_template("feed.html", tweets=database.get_posts(-1))

@app.route("/login/", methods=["GET", "POST"])
def login():

	if request.method == "GET":
		return render_template("login.html")
	
	login = request.form["login"]
	password = request.form["password"]

	if not login:
		flash("Please enter a login")
		print("NO LOGIN")
	elif not password:
		flash("Please enter a password")
		print("NO PASSWORD")
	else:
		uuid = database.login(login, password)
		if not uuid:
			flash("Incorrect login or password")
		else:
			session["login"] = login
			resp = make_response(redirect("/"))
			resp.set_cookie("login", login)
			return resp

	return render_template("login.html")

if __name__ == "__main__":
	app.run(debug=True)