import initialize, {initSync, CompiledStylesV1} from '../../pkg-css-web/turbo_dom_css.js';
initialize({module_or_path: new Uint8Array()});
initSync({module: new Uint8Array()});
const model = new CompiledStylesV1('{}'); model.dispose();
import privateInitialize from '../../css-wasm/turbo_dom_css.web.mjs';
privateInitialize({module_or_path: new Uint8Array()});
