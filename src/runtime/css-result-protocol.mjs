// The protocol selects the result transport, not a second CSS implementation.
export function validateCssResultProtocol(backend, protocol = 'wire-v1') {
 if(protocol !== 'wire-v1' && protocol !== 'native-state-v1')throw new TypeError('turbo-dom: cssResultProtocol must be "wire-v1" or "native-state-v1"');
 if(protocol === 'native-state-v1' && backend !== 'rust')throw new TypeError('turbo-dom: native-state-v1 requires cssBackend "rust"');
 return protocol === 'native-state-v1';
}
