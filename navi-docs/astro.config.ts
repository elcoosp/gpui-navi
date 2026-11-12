import { defineDocsConfig } from '@elcoosp/docs-toolkit/config';
import starlightLinksValidator from "starlight-links-validator";
import starlightImageZoom from "starlight-image-zoom";

export default await defineDocsConfig({
  llmsTxt: {},
  changelog: { repo: 'elcoosp/navi' },
  site: 'https://navi.vercel.app',
  domain: 'navi.vercel.app',
  title: "Navi",
  description: "A powerful file-based router for Rust UI frameworks — one route table, every frontend, powered by quoin.",
  sidebar: [],
  extraStarlight: {
                        expressiveCode: {
                        	themes: ['github-light', 'github-dark']
                        },
                        title: "Navi",
                        description:
                                "A powerful file-based router for Rust UI frameworks — one route table, every frontend, powered by quoin.",
                        // The horizontal logo already contains the "Navi" wordmark, so it
                        // replaces the text title in the header while staying accessible.

                        logo: { src: './src/assets/logo.png', replacesTitle: true },
                        favicon: '/favicon.png',
                        social: [
                                {
                                        icon: "github",
                                        label: "GitHub",
                                        href: "https://github.com/elcoosp/navi"
                                },
                        ],
                        editLink: {
                                baseUrl:
                                        "https://github.com/elcoosp/navi/edit/main/docs-site/src/content/docs/"
                        },
                                
                        sidebar: [
                                {
                                        label: "Tutorials", translations: { fr: 'Tutoriels' },
                                        items: [{ label: "Getting Started", slug: "tutorials/getting-started" }]
                                },
                                {
                                        label: "Guides", translations: { fr: 'Guides' },
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
                                        ]
                                },
                                {
                                        label: "Reference", translations: { fr: 'Référence' },
                                        items: [
                                                { label: "Workspace Crates", slug: "reference/crates" },
                                                { label: "navi.config.json", slug: "reference/navi-config" },
                                                { label: "define_route!", slug: "reference/define-route" },
                                                { label: "Macros", slug: "reference/macros" },
                                                { label: "Components", slug: "reference/components" },
                                                { label: "Runtime & Router Options", slug: "reference/runtime" },
                                                { label: "Core Types", slug: "reference/core-types" },
                                        ]
                                },
                                {
                                        label: "Explanation", translations: { fr: 'Explication' },
                                        items: [
                                                { label: "Architecture", slug: "explanation/architecture" },
                                                { label: "Framework Agnosticism", slug: "explanation/framework-agnosticism" },
                                                { label: "Route Matching", slug: "explanation/route-matching" },
                                                { label: "Performance Design", slug: "explanation/performance" },
                                        ]
                                },
                        ],
                        // The docs collection ships its own 404 entry
                        // (src/content/docs/404.mdx), which the catch-all
                        // route renders at /404 — Starlight's built-in 404
                        // route is disabled to avoid a route conflict.
                        disable404Route: true,
                        plugins: [starlightLinksValidator(), starlightImageZoom()]
                },
});
