// SoA buffer accessor. Reads tree structure straight from the typed arrays the
// native parser produced — no node objects allocated until something asks.

const NS_SHORT = ['', 'svg', 'math'];

export class Buffer {
  constructor(soa) {
    this.soa = soa;
    this.length = soa.nodeType.length;
  }
  nodeType(i) { return this.soa.nodeType[i]; }
  ns(i) { return NS_SHORT[this.soa.ns[i]] || ''; }
  tagName(i) { return this.soa.tagNames[this.soa.tagId[i]]; }
  parent(i) { return this.soa.parent[i]; }
  firstChild(i) { return this.soa.firstChild[i]; }
  nextSib(i) { return this.soa.nextSib[i]; }
  text(i) { const t = this.soa.textId[i]; return t < 0 ? '' : this.soa.strings[t]; }
  publicId(i) { const t = this.soa.pubId[i]; return t < 0 ? '' : this.soa.strings[t]; }
  systemId(i) { const t = this.soa.sysId[i]; return t < 0 ? '' : this.soa.strings[t]; }
  attrs(i) {
    const start = this.soa.attrStart[i];
    if (start < 0) return [];
    const count = this.soa.attrCount[i];
    const out = new Array(count);
    for (let k = 0; k < count; k++) {
      out[k] = {
        name: this.soa.attrName[start + k],
        value: this.soa.attrValue[start + k],
        prefix: this.soa.attrPrefix[start + k] || '',
      };
    }
    return out;
  }
}
