(async () => {
	const results = [];
	for (const kind of ["css", "waapi"])
		for (const action of [-500, 500, 1500, 2500, "finish", "cancel", "idle"]) {
			const frame = document.createElement("iframe");
			const ready = new Promise((resolve) => (frame.onload = resolve));
			frame.srcdoc = `<!doctype html><style>html,body{margin:0}#subject{position:relative;width:100px;height:100px}#inner{position:absolute;inset:0;z-index:100}#cover{position:absolute;left:0;top:0;width:100px;height:100px;z-index:1}${kind === "css" ? "#subject{animation:a 1s 1s linear}@keyframes a{from{opacity:.5}to{opacity:1}}" : ""}</style><div id=subject><div id=inner></div></div><div id=cover></div>`;
			document.body.append(frame);
			await ready;
			const win = frame.contentWindow,
				doc = frame.contentDocument,
				subject = doc.getElementById("subject");
			const animation =
				kind === "css"
					? doc.getAnimations()[0]
					: subject.animate([{ opacity: 0.5 }, { opacity: 1 }], {
							duration: 1000,
							delay: 1000,
							fill: "none",
						});
			if (action === "finish") animation.finish();
			else if (action === "cancel" || action === "idle") animation.cancel();
			else {
				animation.pause();
				animation.currentTime = action;
			}
			await new Promise((resolve) =>
				win.requestAnimationFrame(() => win.requestAnimationFrame(resolve)),
			);
			results.push({
				kind,
				action,
				hit: doc.elementFromPoint(50, 50)?.id,
				opacity: win.getComputedStyle(subject).opacity,
				state: animation.playState,
				time: animation.currentTime,
			});
			frame.remove();
		}
	return results;
})();
