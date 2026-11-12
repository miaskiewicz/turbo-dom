(() => {
	const touch = { clientX: 10, clientY: 10 };
	return ["touches", "targetTouches", "changedTouches"].map((name) => {
		const event = new TouchEvent("touchstart", { [name]: [touch] });
		return {
			name,
			length: event[name].length,
			same: event[name][0] === touch,
			clientX: event[name][0].clientX,
			clientY: event[name][0].clientY,
		};
	});
})();
