/**
 * Logo optimization pipeline (SVGO Node API).
 *
 * Optimizes the Navi logo assets committed to the docs site. Guarantees:
 *   - `viewBox` must survive — without it the SVG cannot scale and will be
 *     cropped or blurry.
 *   - The Rust-orange gradient stop (#ff6b2b) must survive.
 *   - Every `url(#id)` reference must resolve to an id in the same document.
 *     svgo 4 minifies ids (gradRust -> a) and rewrites the references
 *     consistently, which is fine; dangling references are not.
 */
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url)); // docs-site/scripts
const root = path.resolve(here, ".."); // docs-site/
const require = createRequire(path.join(root, "package.json"));
const { optimize } = require("svgo");

const FILES = [
	path.join(root, "src/assets/logo-light.svg"),
	path.join(root, "src/assets/logo-dark.svg"),
	path.join(root, "public/favicon.svg"),
];

for (const file of FILES) {
	const svg = readFileSync(file, "utf8");
	const out = optimize(svg, {
		multipass: true,
		plugins: ["preset-default"],
	});
	if (!out.data.includes("viewBox=")) {
		throw new Error(`viewBox was stripped from ${file} — refusing to write`);
	}
	if (!/#ff6b2b/i.test(out.data)) {
		throw new Error(`Rust-orange gradient stop lost from ${file} — refusing to write`);
	}
	const refs = [...out.data.matchAll(/url\(#([^)]+)\)/g)].map((m) => m[1]);
	if (refs.length === 0) {
		throw new Error(`gradient reference lost from ${file} — refusing to write`);
	}
	for (const id of refs) {
		if (!out.data.includes(`id="${id}"`)) {
			throw new Error(`dangling url(#${id}) in ${file} — refusing to write`);
		}
	}
	writeFileSync(file, out.data);
	console.log(`${file}: ${svg.length} -> ${out.data.length} bytes, refs: ${refs.join(", ")}`);
}
