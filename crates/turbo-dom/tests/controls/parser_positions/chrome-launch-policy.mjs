// Optional isolation for authored localhost controls; normal browser flags stay unchanged.
export function chromeExtensionArgs(config) {
  if (config.disableExtensions !== true) return [];
  const host = new URL(config.url).hostname;
  if (!["localhost", "127.0.0.1", "[::1]"].includes(host))
    throw new TypeError("disableExtensions is restricted to authored localhost controls");
  return ["--disable-extensions", "--disable-component-extensions-with-background-pages"];
}
