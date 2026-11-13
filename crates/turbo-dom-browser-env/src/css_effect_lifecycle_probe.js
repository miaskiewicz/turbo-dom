(async () => {
	const message = document.getElementById("message");
	const animation = message.getAnimations()[0];
	await animation.finished;
	const finite = {
		name: animation.animationName,
		state: animation.playState,
		progress: animation.effect.getComputedTiming().progress,
		count: message.getAnimations().length,
		width: document.getElementById("feedback").getBoundingClientRect().width,
	};
	const states = [];
	for (const value of [
		"fadeInUp .3s infinite",
		"fadeInUp .3s paused",
		"fadeInUp .3s forwards",
		"fadeInUp .3s both",
	]) {
		message.style.animation = "none";
		message.getAnimations();
		message.style.animation = value;
		const current = message.getAnimations()[0];
		if (!value.includes("infinite") && !value.includes("paused"))
			await current.finished;
		const timing = current.effect.getComputedTiming();
		states.push({
			declaration: value,
			state: current.playState,
			fills: timing.progress !== null,
			finite: timing.activeDuration !== Infinity,
		});
	}
	return { finite, states };
})();
