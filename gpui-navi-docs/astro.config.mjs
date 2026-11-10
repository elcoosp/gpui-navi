// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import astroTakumi from 'astro-takumi';
import fs from 'node:fs';
import { makeRender } from './src/og-render.mjs';
import starlightLinksValidator from "starlight-links-validator";
import starlightImageZoom from "starlight-image-zoom";

const __logoB64 = fs.readFileSync("./src/assets/logo.png").toString("base64");
const __render = makeRender({
  logoDataUrl: "data:image/png;base64," + __logoB64,
  accentHex: "#743e3e",
  accentHexA12: "#743e3e1a",
  domain: "gpui-navi.vercel.app",
  projectName: "Navi",
});

export default defineConfig({
	vite: {
		ssr: { external: ["jsdom", "astro-takumi"] },
	},
        // Conventional GitHub Pages origin for the repo — see
        // docs/decisions/0006-site-url.md. Update robots.txt when this changes.
        site: 'https://gpui-navi.vercel.app',
        integrations: [
                starlight({
			routeMiddleware: "./src/routeData.ts",
                        expressiveCode: {
                        	themes: ['github-light', 'github-dark'],
                        },
                        title: "Navi",
                        description:
                                "A powerful file-based router for Rust UI frameworks — one route table, every frontend, powered by quoin.",
                        // The horizontal logo already contains the "Navi" wordmark, so it
                        // replaces the text title in the header while staying accessible.

                        logo: { src: './src/assets/logo.png', replacesTitle: true },
                        favicon: '/favicon.png',
                        customCss: ["./src/styles/custom.css"],
                        social: [
                                {
                                        icon: "github",
                                        label: "GitHub",
                                        href: "https://github.com/elcoosp/gpui-navi",
                                },
                        ],
                        editLink: {
                                baseUrl:
                                        "https://github.com/elcoosp/gpui-navi/edit/main/docs-site/src/content/docs/",
                        },
                        defaultLocale: "root",
                        locales: {
                                root: { label: "English", lang: "en" },
                        },
                        sidebar: [
                                {
                                        label: "Tutorials",
                                        items: [{ label: "Getting Started", slug: "tutorials/getting-started" }],
                                },
                                {
                                        label: "Guides",
                                        items: [
                                                { label: "Installation", slug: "guides/installation" },
                                                { label: "File-Based Routing", slug: "guides/file-based-routing" },
                                                { label: "Loaders & Suspense", slug: "guides/loaders-and-suspense" },
                                                { label: "Route Guards", slug: "guides/route-guards" },
                                                { label: "Navigation Blockers", slug: "guides/navigation-blockers" },
                                                { label: "Search Params", slug: "guides/search-params" },
                                                { label: "Validation", slug: "guides/validation" },
                                                { label: "Link Preloading", slug: "guides/preloading" },
                                                { label: "Scroll Restoration", slug: "guides/scroll-restoration" },
                                                { label: "Devtools", slug: "guides/devtools" },
                                        ],
                                },
                                {
                                        label: "Reference",
                                        items: [
                                                { label: "Workspace Crates", slug: "reference/crates" },
                                                { label: "navi.config.json", slug: "reference/navi-config" },
                                                { label: "define_route!", slug: "reference/define-route" },
                                                { label: "Macros", slug: "reference/macros" },
                                                { label: "Components", slug: "reference/components" },
                                                { label: "Runtime & Router Options", slug: "reference/runtime" },
                                                { label: "Core Types", slug: "reference/core-types" },
                                        ],
                                },
                                {
                                        label: "Explanation",
                                        items: [
                                                { label: "Architecture", slug: "explanation/architecture" },
                                                { label: "Framework Agnosticism", slug: "explanation/framework-agnosticism" },
                                                { label: "Route Matching", slug: "explanation/route-matching" },
                                                { label: "Performance Design", slug: "explanation/performance" },
                                        ],
                                },
                        ],
                        // The docs collection ships its own 404 entry
                        // (src/content/docs/404.mdx), which the catch-all
                        // route renders at /404 — Starlight's built-in 404
                        // route is disabled to avoid a route conflict.
                        disable404Route: true,
                        plugins: [starlightLinksValidator(), starlightImageZoom()],
                }),
        
		astroTakumi({
			options: {
				domBackend: "linkedom",
				fonts: [fs.readFileSync("./node_modules/@fontsource/roboto/files/roboto-latin-400-normal.woff2")],
				width: 1200,
				height: 630,
				format: "png",
			},
			render: __render,
		}),
	],
});
