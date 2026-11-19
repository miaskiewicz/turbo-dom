import { inflateSync } from "node:zlib";

function paeth(left, above, upperLeft) {
	const estimate = left + above - upperLeft;
	const distances = [
		Math.abs(estimate - left),
		Math.abs(estimate - above),
		Math.abs(estimate - upperLeft),
	];
	const nearest = Math.min(...distances);
	return [left, above, upperLeft][distances.indexOf(nearest)];
}
const prediction = [
	() => 0,
	(left) => left,
	(_left, above) => above,
	(left, above) => Math.floor((left + above) / 2),
	paeth,
];

// Independent PNG scanline decoding for deterministic raster color gates.
export function pngPixels(png) {
	const width = png.readUInt32BE(16),
		height = png.readUInt32BE(20);
	const channels = { 2: 3, 6: 4 }[png[25]];
	if (png[24] !== 8 || !channels)
		throw new Error("Expected 8-bit RGB/RGBA PNG");
	const chunks = [];
	for (let offset = 8; offset < png.length; ) {
		const length = png.readUInt32BE(offset);
		if (png.toString("ascii", offset + 4, offset + 8) === "IDAT")
			chunks.push(png.subarray(offset + 8, offset + 8 + length));
		offset += length + 12;
	}
	const rows = inflateSync(Buffer.concat(chunks));
	const stride = width * channels,
		pixels = Buffer.alloc(stride * height);
	for (let row = 0; row < height; row++) {
		const predictor = prediction[rows[row * (stride + 1)]];
		if (!predictor) throw new Error("Unsupported PNG row filter");
		for (let column = 0; column < stride; column++) {
			const index = row * stride + column;
			const left = column < channels ? 0 : pixels[index - channels];
			const above = row === 0 ? 0 : pixels[index - stride];
			const upperLeft =
				row === 0 || column < channels ? 0 : pixels[index - stride - channels];
			pixels[index] =
				(rows[row * (stride + 1) + 1 + column] +
					predictor(left, above, upperLeft)) &
				255;
		}
	}
	return {
		width,
		height,
		pixel: (x, y) => [
			...pixels.subarray(
				(y * width + x) * channels,
				(y * width + x) * channels + channels,
			),
		],
	};
}
