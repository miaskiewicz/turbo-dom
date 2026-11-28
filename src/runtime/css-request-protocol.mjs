// Input transport only; all modes use the same Rust CSS implementation.
export function validateCssRequestProtocol(backend, request = 'wire-v1', result = 'wire-v1') {
 if(request !== 'wire-v1' && request !== 'context-template-v1')throw new TypeError('turbo-dom: cssRequestProtocol must be "wire-v1" or "context-template-v1"');
 if(request === 'wire-v1')return false;
 if(backend !== 'rust')throw new TypeError('turbo-dom: context-template-v1 requires cssBackend "rust"');
 if(result !== 'wire-v1')throw new TypeError('turbo-dom: context-template-v1 requires cssResultProtocol "wire-v1"; native-state-v1 combination is not supported');
 return true;
}
