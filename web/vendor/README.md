# Vendored browser dependencies

`lua-runtime.js` needs [wasmoon](https://github.com/ceifa/wasmoon) to run Lua
plugins in the browser. Both files are loaded as classic scripts, so they are
checked in here instead of being fetched at build time.

- `wasmoon.js` — `wasmoon@1.16.0/dist/index.js` (UMD build, exposes `window.wasmoon`)
- `glue.wasm` — `wasmoon@1.16.0/dist/glue.wasm`
- `LICENSE.wasmoon` — MIT license of the package

To update, run:

```sh
npm pack wasmoon@<version>
tar xzf wasmoon-<version>.tgz
cp package/dist/index.js web/vendor/wasmoon.js
cp package/dist/glue.wasm web/vendor/glue.wasm
cp package/LICENSE web/vendor/LICENSE.wasmoon
```

Keep the version in sync with the `LuaFactory` API used by `web/lua-runtime.js`.
