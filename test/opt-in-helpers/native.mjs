import {createRequire} from 'node:module';
const require = createRequire(import.meta.url);
const {hostPlatform} = require('../../scripts/check-css-host.cjs');
export const nativeSuffix = hostPlatform(process);
export const nativeCssFilename = `turbo-dom-css.${nativeSuffix}.node`;
export const nativeParserFilename = `turbo-dom-parser.${nativeSuffix}.node`;
export function loadNativeCss() { return require(`../../native-css/${nativeCssFilename}`); }
