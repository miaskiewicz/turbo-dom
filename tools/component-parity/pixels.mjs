import { deflateSync } from "node:zlib";
import { pngPixels } from "./png-pixels.mjs";

function crc32(bytes) {
	let crc = 0xffffffff;
	for (const byte of bytes) {
		crc ^= byte;
		for (let bit = 0; bit < 8; bit++)
			crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
	}
	return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
	const body = Buffer.concat([Buffer.from(type), data]);
	const length = Buffer.alloc(4);
	length.writeUInt32BE(data.length);
	const crc = Buffer.alloc(4);
	crc.writeUInt32BE(crc32(body));
	return Buffer.concat([length, body, crc]);
}

export function encodePng(width, height, rows) {
	const header = Buffer.alloc(13);
	header.writeUInt32BE(width);
	header.writeUInt32BE(height, 4);
	header[8] = 8;
	header[9] = 2;
	return Buffer.concat([
		Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
		chunk("IHDR", header),
		chunk("IDAT", deflateSync(rows)),
		chunk("IEND", Buffer.alloc(0)),
	]);
}

// Pixel error is diagnostic: text antialiasing is not a layout failure.
export function pixelDiff(expected, actual, threshold = 16) {
	const left = pngPixels(expected);
	const right = pngPixels(actual);
	if (left.width !== right.width || left.height !== right.height)
		throw new Error(
			"Compare captures with identical viewport and device scale",
		);
	const rows = Buffer.alloc((left.width * 3 + 1) * left.height);
	let changed = 0;
	let totalError = 0;
	for (let y = 0; y < left.height; y++) {
		for (let x = 0; x < left.width; x++) {
			const a = left.pixel(x, y);
			const b = right.pixel(x, y);
			const errors = a
				.slice(0, 3)
				.map((value, index) => Math.abs(value - b[index]));
			const different = Math.max(...errors) > threshold;
			changed += Number(different);
			totalError += errors.reduce((sum, error) => sum + error, 0);
			const offset = y * (left.width * 3 + 1) + 1 + x * 3;
			rows.set(different ? [255, 0, 128] : [255, 255, 255], offset);
		}
	}
	const pixels = left.width * left.height;
	return {
		changedPixels: changed,
		changedFraction: changed / pixels,
		meanChannelError: totalError / (pixels * 3),
		png: encodePng(left.width, left.height, rows),
	};
}
