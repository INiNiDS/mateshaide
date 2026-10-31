(function (root) {
    'use strict';
    const runtimes = new Map();
    let nextId = 1;
    const script = typeof document !== 'undefined' ? document.currentScript : null;
    const base = script ? new URL('.', script.src) : null;
    let module;
    function install(luaModule, api) {
        module = luaModule;
        root.MateshaLua = {
            create(host, jsonSource, bootstrapSource) {
                if (runtimes.size >= 128) throw Error('Lua VM count limit');
                const engine = new api.LuaEngine(module, {
                    openStandardLibs: true, injectObjects: false,
                    enableProxy: false, traceAllocations: true,
                });
                try {
                    engine.global.setMemoryMax(64 * 1024 * 1024);
                    engine.global.set('__mateshaHost', host);
                    engine.global.set('__mateshaClock', () => performance.now());
                    engine.doStringSync(`
                        local json = (function() ${jsonSource}\nend)()
                        local factory = (function() ${bootstrapSource}\nend)()
                        local function hook(enabled, callback)
                            if enabled then debug.sethook(callback, '', 1000) else debug.sethook() end
                        end
                        __mateshaInvoke = factory(json, __mateshaHost, __mateshaClock, hook)
                    `);
                    const invoke = engine.global.get('__mateshaInvoke');
                    const id = nextId++;
                    runtimes.set(id, { engine, invoke });
                    return id;
                } catch (error) {
                    engine.global.close(); throw error;
                }
            },
            invoke(id, files, entry, state, name, args) {
                const vm = runtimes.get(id);
                if (!vm) throw Error('Lua VM is closed');
                return vm.invoke(files, entry, state, name, args);
            },
            destroy(id) {
                const vm = runtimes.get(id);
                if (vm) { runtimes.delete(id); vm.engine.global.close(); }
            },
        };
    }
    root.MateshaLuaInstall = install;
    if (typeof document !== 'undefined') {
        root.MateshaLuaReady = (async () => {
            if (!root.wasmoon) throw Error('Load web/vendor/wasmoon.js before web/lua-runtime.js');
            const factory = new root.wasmoon.LuaFactory(new URL('vendor/glue.wasm', base).href);
            const luaModule = await factory.getLuaModule();
            install(luaModule, root.wasmoon);
        })();
    }
})(globalThis);
