(() => {
	const rows = {};
	const host = () => {
		const node = document.createElement("p");
		document.body.append(node);
		return node;
	};
	const point = (range) => [
		range.startContainer.nodeName,
		range.startOffset,
		range.endContainer.nodeName,
		range.endOffset,
		String(range),
	];
	{
		const node = host();
		node.textContent = "abcdef";
		const text = node.firstChild,
			range = document.createRange();
		range.setStart(text, 2);
		range.setEnd(text, 5);
		const states = [];
		text.insertData(1, "ZZ");
		states.push(point(range));
		text.deleteData(3, 2);
		states.push(point(range));
		text.replaceData(0, 4, "X");
		states.push(point(range));
		text.data = "fresh";
		states.push(point(range));
		rows.character = states;
		node.remove();
	}
	{
		const node = host();
		node.textContent = "abcdef";
		const text = node.firstChild,
			range = document.createRange(),
			outside = document.createRange();
		range.setStart(text, 1);
		range.setEnd(text, 5);
		outside.setStart(node, 1);
		outside.collapse(true);
		const next = text.splitText(3);
		rows.split = [
			point(range),
			range.endContainer === next,
			point(outside),
			node.innerHTML,
		];
		node.remove();
	}
	{
		const node = host();
		node.innerHTML = "<i>a</i><b>bc</b><u>d</u>";
		const range = document.createRange();
		range.setStart(node.children[1].firstChild, 1);
		range.setEnd(node, 3);
		node.children[1].remove();
		rows.removal = [point(range), node.innerHTML];
		node.remove();
	}
	{
		const node = host();
		node.innerHTML = "<i>a</i><b>b</b><u>c</u>";
		const range = document.createRange();
		range.setStart(node, 1);
		range.setEnd(node, 3);
		const inserted = document.createElement("em");
		inserted.textContent = "x";
		node.insertBefore(inserted, node.children[1]);
		const states = [point(range)];
		const fragment = document.createDocumentFragment();
		fragment.append(document.createTextNode("y"), document.createTextNode("z"));
		node.insertBefore(fragment, node.firstChild);
		states.push(point(range));
		node.appendChild(node.children[0]);
		states.push(point(range));
		rows.insertion = [states, node.innerHTML];
		node.remove();
	}
	{
		const node = host();
		node.innerHTML = "<b>abc</b><i>def</i>";
		const range = document.createRange();
		range.setStart(node.firstChild.firstChild, 1);
		range.setEnd(node.lastChild.firstChild, 2);
		const clone = range.cloneRange();
		node.textContent = "new";
		rows.replacement = [point(range), point(clone), node.innerHTML];
		node.remove();
	}
	{
		const node = host();
		node.innerHTML = "<b>abc</b><i>def</i>";
		const range = document.createRange();
		range.selectNodeContents(node);
		node.innerHTML = "<em>new</em>";
		rows.markup = [point(range), node.innerHTML];
		node.remove();
	}
	{
		const node = host();
		node.textContent = "a😀bc";
		const text = node.firstChild,
			range = document.createRange();
		range.setStart(text, 3);
		range.setEnd(text, 5);
		text.replaceData(1, 2, "X");
		rows.utf16 = [point(range), text.data, text.substringData(1, 2)];
		node.remove();
	}
	{
		const node = host(),
			first = document.createTextNode("ab"),
			empty = document.createTextNode(""),
			last = document.createTextNode("cd");
		node.append(first, empty, last);
		const range = document.createRange(),
			parentRange = document.createRange();
		range.setStart(last, 1);
		range.setEnd(last, 2);
		parentRange.setStart(node, 2);
		parentRange.setEnd(node, 3);
		node.normalize();
		rows.normalize = [
			point(range),
			point(parentRange),
			range.startContainer === first,
			node.childNodes.length,
			node.innerHTML,
		];
		node.remove();
	}
	{
		const node = host();
		node.innerHTML = "<b>abc</b><i>def</i><u>ghi</u>";
		const inside = document.createRange(),
			around = document.createRange();
		inside.setStart(node.children[0].firstChild, 1);
		inside.setEnd(node.children[0].firstChild, 2);
		around.selectNode(node.children[0]);
		const moved = node.children[0];
		node.moveBefore(moved, null);
		rows.move = [point(inside), point(around), node.innerHTML];
		node.remove();
	}
	{
		const node = host();
		node.innerHTML = "<b>abc</b><i>def</i>";
		const range = document.createRange();
		range.setStart(node.firstChild.firstChild, 1);
		range.setEnd(node, 2);
		document.adoptNode(node.firstChild);
		rows.adoption = [point(range), node.innerHTML];
		node.remove();
	}
	{
		const states = [];
		for (const position of [
			"beforebegin",
			"afterend",
			"afterbegin",
			"beforeend",
		]) {
			const node = host();
			node.innerHTML = "<i>a</i><b>b</b><u>c</u>";
			const range = document.createRange(),
				inside = document.createRange();
			range.setStart(node, 1);
			range.setEnd(node, 3);
			inside.selectNodeContents(node.children[1].firstChild);
			node.children[1].insertAdjacentHTML(position, "<em>x</em>y");
			states.push([position, point(range), point(inside), node.innerHTML]);
			node.remove();
		}
		rows.adjacentMarkup = states;
	}
	{
		const node = host();
		node.innerHTML = "<i>a</i><b>bc</b><u>d</u>";
		const around = document.createRange(),
			inside = document.createRange(),
			after = document.createRange();
		around.selectNode(node.children[1]);
		inside.selectNodeContents(node.children[1].firstChild);
		after.setStart(node, 3);
		after.collapse(true);
		node.children[1].outerHTML = "<em>x</em>y";
		rows.outerMarkup = [
			point(around),
			point(inside),
			point(after),
			node.innerHTML,
		];
		node.remove();
	}
	{
		const node = host();
		node.innerHTML = "<i>a</i><b>bc</b><u>d</u>";
		const around = document.createRange(),
			inside = document.createRange();
		around.selectNodeContents(node);
		inside.selectNodeContents(node.children[1].firstChild);
		node.innerText = "new\ntext";
		rows.innerText = [point(around), point(inside), node.innerHTML];
		node.remove();
	}
	{
		const states = [];
		for (const value of ["X", "A\nB", ""]) {
			const node = host();
			node.innerHTML = "left<b>old</b>right";
			const previous = document.createRange(),
				inside = document.createRange(),
				next = document.createRange(),
				around = document.createRange();
			previous.selectNodeContents(node.firstChild);
			inside.selectNodeContents(node.children[0].firstChild);
			next.selectNodeContents(node.lastChild);
			around.setStart(node, 1);
			around.setEnd(node, 3);
			node.children[0].outerText = value;
			states.push([
				value,
				point(previous),
				point(inside),
				point(next),
				point(around),
				node.innerHTML,
			]);
			node.remove();
		}
		rows.outerText = states;
	}
	{
		const text = document.createTextNode("abc"),
			range = document.createRange();
		range.selectNodeContents(text);
		const errors = [];
		for (const run of [
			() => text.insertData(4, "x"),
			() => text.deleteData(-1, 1),
			() => text.replaceData(4, 1, "x"),
			() => text.substringData(4, 1),
			() => text.splitText(4),
			() => text.insertData(Symbol(), "x"),
		])
			try {
				run();
				errors.push("missing");
			} catch (error) {
				errors.push(error.name);
			}
		rows.errors = [errors, text.data, point(range)];
	}
	return rows;
})();
