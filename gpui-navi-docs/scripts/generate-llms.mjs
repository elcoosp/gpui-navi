/**
 * Generates dist/llms.txt and dist/llms-full.txt after `astro build`.
 *
 * llms.txt      — a curated index: site summary plus every docs page with
 *                 its title, one-line description, and URL.
 * llms-full.txt — the full content of every docs page concatenated,
 *                 with frontmatter and MDX import lines cleaned up so the
 *                 result is plain, agent-readable Markdown.
 *
 * Both files land at the site root (dist/), so they are served at /llms.txt
 * and /llms-full.txt alongside the pages they index.
 */
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const DOCS = "src/content/docs";
const OUT_DIR = "dist";
const SITE_TITLE = "Navi";
const SITE_DESCRIPTION =
        "A powerful file-based router for Rust UI frameworks — one route table, every frontend, powered by quoin.";
const SITE_URL = "https://github.com/elcoosp/gpui-navi";

async function walk(dir) {
        const entries = await readdir(dir, { withFileTypes: true });
        const files = [];
        for (const e of entries) {
                const p = path.join(dir, e.name);
                if (e.isDirectory()) files.push(...(await walk(p)));
                else if (/\.(md|mdx)$/.test(e.name)) files.push(p);
        }
        return files.sort();
}

function parseFrontmatter(text) {
        const match = /^---\r?\n([\s\S]*?)\r?\n---/.exec(text);
        if (!match) return { data: {}, body: text };
        const data = {};
        for (const line of match[1].split(/\r?\n/)) {
                const m = /^(title|description|type):\s*(.+)$/.exec(line.trim());
                if (m) data[m[1]] = m[2].replace(/^["']|["']$/g, "");
        }
        return { data, body: text.slice(match[0].length) };
}

function toMarkdownBody(body) {
        return (
                body
                        // Drop MDX import lines and JSX component usages that carry no
                        // information once stripped of their tags.
                        .replace(/^import\s+.*;?\s*$/gm, "")
                        .replace(/<\/?(Card|CardGrid|LinkCard|Tabs|TabItem|Steps|FileTree|Aside|Badge)[^>]*>/g, "")
                        .replace(/\n{3,}/g, "\n\n")
                        .trim()
        );
}

const files = await walk(DOCS);
const pages = [];
for (const file of files) {
        const raw = await readFile(file, "utf8");
        const { data, body } = parseFrontmatter(raw);
        const rel = path.relative(DOCS, file);
        const slug = rel.replace(/\.(md|mdx)$/, "");
        const route =
                slug === "index"
                        ? "/"
                        : `/${slug.split("/").slice(0, -1).join("/")}/${slug.split("/").pop()}/`.replace(/\/index\/$/, "/");
        pages.push({
                route,
                title: data.title ?? slug,
                description: data.description ?? "",
                type: data.type ?? "",
                body: toMarkdownBody(body),
                rel,
        });
}

const bySection = (t) => pages.filter((p) => p.rel.startsWith(`${t}/`));
const sections = [
        ["Tutorials", "tutorials"],
        ["Guides", "guides"],
        ["Reference", "reference"],
        ["Explanation", "explanation"],
];

let llms = `# ${SITE_TITLE}\n\n${SITE_DESCRIPTION}\n\nSource: ${SITE_URL}\n`;
for (const [label, dir] of sections) {
        const group = bySection(dir);
        if (!group.length) continue;
        llms += `\n## ${label}\n\n`;
        for (const p of group) {
                llms += `- [${p.title}](${p.route}): ${p.description}\n`;
        }
}

let full = `# ${SITE_TITLE} — full documentation\n\n${SITE_DESCRIPTION}\n\nSource: ${SITE_URL}\n\n`;
for (const [label, dir] of sections) {
        const group = bySection(dir);
        if (!group.length) continue;
        full += `<!-- section: ${label} -->\n\n`;
        for (const p of group) {
                full += `---\n\n# ${p.title}\n\n${p.description ? `${p.description}\n\n` : ""}${p.body}\n\n`;
        }
}
const landing = pages.find((p) => p.route === "/");
if (landing) {
        full += `---\n\n# ${landing.title}\n\n${landing.body}\n`;
}

await writeFile(path.join(OUT_DIR, "llms.txt"), llms);
await writeFile(path.join(OUT_DIR, "llms-full.txt"), full);
console.log(`llms.txt: ${pages.length} pages indexed; llms-full.txt: ${full.length} bytes`);
