// Types for `@miaskiewicz/turbo-dom/runtime` (src/runtime/index.mjs).
//
// The runtime classes implement the DOM's shape but are not the lib.dom classes, so
// they are typed loosely here (members are `any`); the entry points are typed exactly.

/** A parser binding (native napi, wasm, or an injected one). */
export interface ParserBinding {
  parse(html: string): unknown;
  parseBuffer(html: string): unknown;
  parseFragment(html: string, context?: string): unknown;
  [key: string]: unknown;
}

export type ParserMode = 'wasm' | 'native' | 'auto';

export type CustomElementState = 'undefined' | 'failed' | 'uncustomized' | 'precustomized' | 'custom';

export interface EnvironmentOptions {
  /** Parser selection, applied process-wide before the first parse. */
  parser?: ParserMode;
  /** The document URL (default `http://localhost/`). */
  url?: string;
}

export interface Environment {
  /** The lazy window (globals are created on first access). */
  window: any;
  document: Document;
  /** The global names the tests have touched since the last reset. */
  touched: Set<string>;
  /** Every global name the window can provide. */
  globalKeys: string[];
  /** Re-point the document at a fresh (or new) parse and drop per-test state. */
  reset(html?: string): void;
}

export declare function createEnvironment(html?: string, options?: EnvironmentOptions): Environment;

/** Inject a ready parser binding; `null` clears it. */
export declare function setParser(binding: ParserBinding | null): void;
/** Force a parser mode; `null` consults the environment again. */
export declare function setParserMode(mode: ParserMode | null): void;
export declare function getParser(): ParserBinding;

/** Install a virtual clock (milliseconds); anything but a function restores the real one. */
export declare function setClock(fn: (() => number) | null): void;

export declare function customElementState(el: unknown): CustomElementState;
/** Record an element's custom element state; returns whether it changed. */
export declare function setCustomElementState(el: unknown, state: CustomElementState): boolean;

/** Parse an HTML string into a new Document. */
export declare function parseDocument(html: string): Document;

export declare const ELEMENT_NODE: 1;
export declare const TEXT_NODE: 3;
export declare const COMMENT_NODE: 8;
export declare const DOCUMENT_NODE: 9;
export declare const DOCUMENT_TYPE_NODE: 10;
export declare const DOCUMENT_FRAGMENT_NODE: 11;

export declare class Event {
  constructor(type: string, init?: { bubbles?: boolean; cancelable?: boolean; composed?: boolean });
  readonly type: string;
  [key: string]: any;
}
export declare class CustomEvent<T = any> extends Event {
  constructor(type: string, init?: { bubbles?: boolean; cancelable?: boolean; composed?: boolean; detail?: T });
  readonly detail: T;
}

export declare class Node {
  readonly nodeType: number;
  readonly nodeName: string;
  [key: string]: any;
}
export declare class Text extends Node {}
export declare class Comment extends Node {}
export declare class DocumentType extends Node {}
export declare class Element extends Node {}
export declare class SVGElement extends Element {}
export declare class MathMLElement extends Element {}
export declare class DocumentFragment extends Node {}
export declare class ShadowRoot extends DocumentFragment {}
export declare class Document extends Node {
  constructor();
}

export declare class DOMParser {
  parseFromString(str: string, type?: string): Document;
}
export declare class XMLSerializer {
  serializeToString(node: Node): string;
}
export declare class MutationObserver {
  constructor(callback: (records: any[], observer: MutationObserver) => void);
  observe(target: Node, options?: Record<string, unknown>): void;
  disconnect(): void;
  takeRecords(): any[];
}
