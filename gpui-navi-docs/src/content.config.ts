import { defineCollection } from "astro:content";
import { docsLoader, i18nLoader } from "@astrojs/starlight/loaders";
import { docsSchema, i18nSchema } from "@astrojs/starlight/schema";

const docs = defineCollection({
        loader: docsLoader(),
        schema: docsSchema(),
});

// Starlight's optional UI-translation collection. Defining it (even with a
// single en.json) keeps the content sync from warning about a missing or
// empty "i18n" collection.
const i18n = defineCollection({
        loader: i18nLoader(),
        schema: i18nSchema(),
});

export const collections = { docs, i18n };
