// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLinksValidator from "starlight-links-validator";
import starlightImageZoom from "starlight-image-zoom";

export default defineConfig({
        // Conventional GitHub Pages origin for the repo — see
        // docs/decisions/0006-site-url.md. Update robots.txt when this changes.
        site: "https://elcoosp.github.io/gpui-navi/",
        integrations: [
                starlight({
                        title: "Navi",
                        description:
                                "A powerful file-based router for Rust UI frameworks — one route table, every frontend, powered by quoin.",
                        // The horizontal logo already contains the "Navi" wordmark, so it
                        // replaces the text title in the header while staying accessible.
                        logo: {
                                light: "./src/assets/logo-light.svg",
                                dark: "./src/assets/logo-dark.svg",
                                replacesTitle: true,
                        },
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
        ],
});
