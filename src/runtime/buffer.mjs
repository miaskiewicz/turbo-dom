// SoA buffer accessor. Reads tree structure straight from the typed arrays the
// native parser produced — no node objects allocated until something asks.

const NS_SHORT = ['', 'svg', 'math'];

// Inflate the packed byte blob into named typed-array views (zero-copy).
// Mirrors the column order in lib.rs JsSoa::from. One ArrayBuffer, ~13 views.
// Exported so the parse cache can unpack ONCE per HTML (the views are read-only
// over the shared immutable buffer) instead of re-unpacking per Document.
export function unpack(soa) {
  const u8 = soa.packed, ab = u8.buffer, base = u8.byteOffset, n = soa.n, m = soa.m;
  let off = base;
  const i32 = () => { const a = new Int32Array(ab, off, n); off += n * 4; return a; };
  const u32n = () => { const a = new Uint32Array(ab, off, n); off += n * 4; return a; };
  const u32m = () => { const a = new Uint32Array(ab, off, m); off += m * 4; return a; };
  const tagId = u32n(), parent = i32(), firstChild = i32(), nextSib = i32(),
    textId = i32(), pubId = i32(), sysId = i32(), attrStart = i32();
  const attrNameId = u32m(), attrValueId = u32m(), attrPrefixId = u32m();
  const attrCount = new Uint16Array(ab, off, n); off += n * 2;
  const nodeType = new Uint8Array(ab, off, n); off += n;
  const ns = new Uint8Array(ab, off, n); off += n;
  // Decode the five string tables from the raw byte blob (one pass per string),
  // done ONCE here per cached parse. The native side ships bytes with no per-string
  // napi UTF-8→UTF-16 conversion; the accessors below read plain arrays (hot path
  // unchanged). meta = [5 counts, then every string's byte length in table order].
  const meta = soa.strMeta, blob = soa.strBlob;
  let mi = 5, bo = 0;
  const table = (count) => {
    const arr = new Array(count);
    for (let k = 0; k < count; k++) {
      const len = meta[mi++];
      arr[k] = len === 0 ? '' : STR_DECODER.decode(blob.subarray(bo, bo + len));
      bo += len;
    }
    return arr;
  };
  const tagNames = table(meta[0]), attrNames = table(meta[1]), attrPrefixes = table(meta[2]),
    attrValues = table(meta[3]), strings = table(meta[4]);
  return {
    nodeType, ns, tagId, parent, firstChild, nextSib, textId, pubId, sysId,
    attrStart, attrCount, attrNameId, attrValueId, attrPrefixId,
    tagNames, attrNames, attrPrefixes, attrValues, strings,
  };
}
// Decode the UTF-8 string blob. Prefer the platform TextDecoder (Node/browser);
// fall back to a tiny inline UTF-8 decoder so turbo-dom loads in a bare V8 with no
// web-platform globals (the native/Node path keeps using the real TextDecoder —
// this branch is never taken there).
export function utf8Decode(bytes) {
  let out = '', i = 0;
  const n = bytes.length;
  while (i < n) {
    const c = bytes[i++];
    if (c < 0x80) out += String.fromCharCode(c);
    else if (c < 0xe0) out += String.fromCharCode(((c & 0x1f) << 6) | (bytes[i++] & 0x3f));
    else if (c < 0xf0) out += String.fromCharCode(((c & 0x0f) << 12) | ((bytes[i++] & 0x3f) << 6) | (bytes[i++] & 0x3f));
    else {
      let cp = ((c & 0x07) << 18) | ((bytes[i++] & 0x3f) << 12) | ((bytes[i++] & 0x3f) << 6) | (bytes[i++] & 0x3f);
      cp -= 0x10000;
      out += String.fromCharCode(0xd800 + (cp >> 10), 0xdc00 + (cp & 0x3ff));
    }
  }
  return out;
}
const STR_DECODER = typeof TextDecoder !== 'undefined' ? new TextDecoder() : { decode: utf8Decode };

export class Buffer {
  constructor(soa) {
    this.soa = soa.packed ? unpack(soa) : soa;
    this.length = this.soa.nodeType.length;
  }
  nodeType(i) { return this.soa.nodeType[i]; }
  ns(i) { return NS_SHORT[this.soa.ns[i]] || ''; }
  tagName(i) { return this.soa.tagNames[this.soa.tagId[i]]; }
  firstChild(i) { return this.soa.firstChild[i]; }
  nextSib(i) { return this.soa.nextSib[i]; }
  text(i) { const t = this.soa.textId[i]; return t < 0 ? '' : this.soa.strings[t]; }
  publicId(i) { const t = this.soa.pubId[i]; return t < 0 ? '' : this.soa.strings[t]; }
  systemId(i) { const t = this.soa.sysId[i]; return t < 0 ? '' : this.soa.strings[t]; }
  // Read a single attr value / presence straight from the columns — no array,
  // no {name,value,prefix} objects. The hot read path (selectors hammer
  // getAttribute('class')/('id')) never materializes the attr list.
  attrGet(i, name) {
    const start = this.soa.attrStart[i];
    if (start < 0) return null;
    const count = this.soa.attrCount[i];
    const { attrNameId, attrNames, attrValueId, attrValues } = this.soa;
    for (let k = 0; k < count; k++) if (attrNames[attrNameId[start + k]] === name) return attrValues[attrValueId[start + k]];
    return null;
  }
  attrHas(i, name) {
    const start = this.soa.attrStart[i];
    if (start < 0) return false;
    const count = this.soa.attrCount[i];
    const { attrNameId, attrNames } = this.soa;
    for (let k = 0; k < count; k++) if (attrNames[attrNameId[start + k]] === name) return true;
    return false;
  }
  attrs(i) {
    const start = this.soa.attrStart[i];
    if (start < 0) return [];
    const count = this.soa.attrCount[i];
    const out = new Array(count);
    const { attrNameId, attrNames, attrValueId, attrValues, attrPrefixId, attrPrefixes } = this.soa;
    for (let k = 0; k < count; k++) {
      out[k] = {
        name: attrNames[attrNameId[start + k]],
        value: attrValues[attrValueId[start + k]],
        prefix: attrPrefixes[attrPrefixId[start + k]] || '',
      };
    }
    return out;
  }
}
