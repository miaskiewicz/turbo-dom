(() => {
  const result = [];
  for (const value of ["1", "auto", "none", "initial", "2 3", "1 0px", "10px", "0", "1 2 30%", "30% 1 2", "content", "0 0", "1.5", "1 1 auto"]) {
    const div = document.createElement("div");
    div.style.flex = value;
    result.push({
      value,
      flex: div.style.flex,
      longhands: [div.style.flexGrow, div.style.flexShrink, div.style.flexBasis],
      cssText: div.style.cssText,
      attribute: div.getAttribute("style"),
    });
  }
  return result;
})();
