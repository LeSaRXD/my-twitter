let like_post = (post_id) => {
	let xmlHttp = new XMLHttpRequest();
	xmlHttp.open("GET", `/like_post/${post_id}`, true);
	xmlHttp.send(null);
};

window.onload = () => {
	let like_buttons = document.querySelectorAll("button.like_button");

	for (let i = 0; i < like_buttons.length; i++) {
		let b = like_buttons.item(i);

		b.addEventListener("click", (e) => {
			e.stopPropagation();

			like_post(b.dataset.id);

			// updating icon and counter
			let icon = b.querySelector(".like_icon");
			let text = b.querySelector(".like_count");
			if (icon.src.includes("/static/like_hollow.png")) {
				icon.src = "/static/like_filled.png";
				text.innerHTML = parseInt(text.innerHTML) + 1;
			} else {
				icon.src = "/static/like_hollow.png";
				text.innerHTML = parseInt(text.innerHTML) - 1;
			}
		});
	}
};
