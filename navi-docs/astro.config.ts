import { defineDocsConfig } from '@elcoosp/docs-toolkit/config';
import starlightChangelogs from 'starlight-changelogs';
import starlightLinksValidator from "starlight-links-validator";
import starlightImageZoom from "starlight-image-zoom";

export default await defineDocsConfig({
  llmsTxt: {},
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
                                        label: "GitHub", translations: { fr: "GitHub" },
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
                                        items: [{ label: "Getting Started", slug: "tutorials/getting-started" }], translations: { fr: "Commencer" }
                                },
                                {
                                        label: "Guides", translations: { fr: 'Guides' },
                                        items: [
                                                { label: "Installation", translations: { fr: "Installation" }, slug: "guides/installation" },
                                                { label: "File-Based Routing", translations: { fr: "Routage basé sur les fichiers" }, slug: "guides/file-based-routing" },
                                                { label: "Loaders & Suspense", translations: { fr: "Chargeurs et Suspense" }, slug: "guides/loaders-and-suspense" },
                                                { label: "Route Guards", translations: { fr: "Gardes de route" }, slug: "guides/route-guards" },
                                                { label: "Navigation Blockers", translations: { fr: "Bloqueurs de navigation" }, slug: "guides/navigation-blockers" },
                                                { label: "Search Params", translations: { fr: "Paramètres de recherche" }, slug: "guides/search-params" },
                                                { label: "Validation", translations: { fr: "Validation" }, slug: "guides/validation" },
                                                { label: "Link Preloading", translations: { fr: "Préchargement des liens" }, slug: "guides/preloading" },
                                                { label: "Scroll Restoration", translations: { fr: "Restauration du défilement" }, slug: "guides/scroll-restoration" },
                                                { label: "Devtools", translations: { fr: "Outils de développement" }, slug: "guides/devtools" },
                                        ]
                                },
                                {
                                        label: "Reference", translations: { fr: 'Référence' },
                                        items: [
                                                { label: "Workspace Crates", translations: { fr: "Crates de l'espace de travail" }, slug: "reference/crates" },
                                                { label: "navi.config.json", translations: { fr: "navi.config.json" }, slug: "reference/navi-config" },
                                                { label: "define_route!", translations: { fr: "define_route!" }, slug: "reference/define-route" },
                                                { label: "Macros", translations: { fr: "Macros" }, slug: "reference/macros" },
                                                { label: "Components", translations: { fr: "Composants" }, slug: "reference/components" },
                                                { label: "Runtime & Router Options", translations: { fr: "Options d'exécution et de routeur" }, slug: "reference/runtime" },
                                                { label: "Core Types", translations: { fr: "Types de base" }, slug: "reference/core-types" },
                                        ]
                                },
                                {
                                        label: "Explanation", translations: { fr: 'Explication' },
                                        items: [
                                                { label: "Architecture", translations: { fr: "Architecture" }, slug: "explanation/architecture" },
                                                { label: "Framework Agnosticism", translations: { fr: "Agnosticisme de framework" }, slug: "explanation/framework-agnosticism" },
                                                { label: "Route Matching", translations: { fr: "Correspondance des routes" }, slug: "explanation/route-matching" },
                                                { label: "Performance Design", translations: { fr: "Conception des performances" }, slug: "explanation/performance" },
                                        ]
                                },
                        ],
                        // The docs collection ships its own 404 entry
                        // (src/content/docs/404.mdx), which the catch-all
                        // route renders at /404 — Starlight's built-in 404
                        // route is disabled to avoid a route conflict.
                        disable404Route: true,
                        plugins: [ starlightChangelogs(), starlightLinksValidator(), starlightImageZoom()]
                },
});
