import {CompiledStylesV1, cssAbiVersion, cssBackendVersion} from '../../pkg-css/turbo_dom_css.js';
const model = new CompiledStylesV1('{}');
const abi: number = cssAbiVersion();
const version: string = cssBackendVersion();
model.metadata(); model.dispose();
import initialize from '../../pkg-css/turbo_dom_css.js';

// @ts-expect-error Node backend namespace is not a web initializer
initialize({});
import * as privateNode from '../../css-wasm/turbo_dom_css.node.cjs';
const directNode = new privateNode.CompiledStylesV1('{}'); directNode.dispose();
