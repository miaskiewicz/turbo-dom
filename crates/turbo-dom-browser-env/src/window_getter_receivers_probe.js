((peer) => {
  const root = globalThis;
  root.name = "root-name";
  peer.name = "child-name";
  const rows = [];
  const receivers = () => [
    ["root", root], ["child", peer], ["object", {}], ["null", null],
    ["undefined", undefined], ["number", 0],
    ["prototype", root.Window.prototype],
    ["forged", Object.create(root.Window.prototype)],
    ["proxy", new Proxy(root, {})],
  ];
  for (const [realm, window] of [["root", root], ["child", peer]]) {
    for (const property of ["closed", "length", "opener", "name"]) {
      const getter = Object.getOwnPropertyDescriptor(window, property).get;
      rows.push({ realm, property, getter });
    }
  }
  const capture = () => rows.map(({realm, property, getter}) => ({
    realm, property,
    receivers: Object.fromEntries(receivers().map(([name, receiver]) => {
      try {
        const value = Reflect.apply(getter, receiver, []);
        return [name, { kind: "return", type: typeof value,
          value: value === undefined ? "undefined" : value === root ? "root" : value === peer ? "child" : value }];
      } catch (error) {
        return [name, { kind: "throw", name: error.name, message: error.message }];
      }
    })),
  }));
  return { live: capture(), removed: capture };
})
