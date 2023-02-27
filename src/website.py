from flask import Flask, Markup, render_template, send_from_directory
from timestamps import format_timestamp
from database import *



app = Flask(__name__, template_folder="../templates", static_folder="../static")
@app.route("/")
def feed():
	return render_template("feed.html", tweets=get_posts(-1))

@app.route("/login/", methods=["GET", "POST"])
def login():
	return render_template("login.html")

@app.route("/static/<path:path>")
def send_static(path):
	return send_from_directory("static", path)

if __name__ == "__main__":
	app.run(debug=True)