# kfind site

The React documentation site uses React Router Framework Mode for the overview,
guides, option reference, internals, benchmarks, and playground. The build
prerenders every document route to `build/client`, then Cloudflare Pages serves
each clean URL as static HTML. Route modules are split, and the playground route
builds the current `kfind-wasm` crate for the browser.
Shared interface and metadata copy uses an i18next catalog. The prerendered
document locale is Korean, and future same-URL locale choices are stored in the
`kfind-document-locale` cookie after hydration.
Benchmark charts come from the approved
`docs/benchmarks/site-morphology.json` snapshot.
The optional smart component resource is stored in the `kfind-assets` R2 bucket
and streamed through a same-origin Pages Function only when requested.

```sh
pnpm --dir site install
pnpm --dir site run dev
pnpm --dir site run build
pnpm --dir site run dev:pages
```

Production is a direct-upload Cloudflare Pages project named `kfind` with
`main` as its production branch. Deployment rebuilds and uploads the component
resource before publishing the static site and Pages Function.

`.github/workflows/pages.yml` deploys every `main` push and supports a manual
run from `main`. The repository must define `CLOUDFLARE_ACCOUNT_ID` and a
`CLOUDFLARE_API_TOKEN` with Pages and R2 write access.

```sh
pnpm --dir site run deploy
```
