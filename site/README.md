# kfind site

The static documentation site builds the current `kfind-wasm` crate for the
browser and copies the approved benchmark charts from `docs/benchmarks`.

```sh
pnpm --dir site install
pnpm --dir site run dev
pnpm --dir site run build
```

Production is a direct-upload Cloudflare Pages project named `kfind` with
`main` as its production branch.

```sh
pnpm --dir site run deploy
```
