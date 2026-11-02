(() => {
  const parent = document.createElement("div");
  const child = document.createElement("span");
  parent.appendChild(child);
  document.body.appendChild(parent);
  const rows = [];
  const read = (name) =>
    rows.push({
      name,
      parent: getComputedStyle(parent).color,
      child: getComputedStyle(child).color,
    });
  read("default");
  parent.style.color = "green";
  read("inherited");
  child.style.color = "inherit";
  read("inherit");
  child.style.color = "initial";
  read("initial");
  child.style.color = "unset";
  read("unset");
  child.style.color = "currentColor";
  read("currentColor");
  child.style.color = "CanvasText";
  read("CanvasText");
  child.style.color = "inherit";
  const saved = getComputedStyle(child);
  parent.style.color = "blue";
  rows.push({ name: "live", color: saved.color });
  parent.remove();
  read("disconnected");
  return rows;
})();
