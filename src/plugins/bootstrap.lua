return function(json, host, clock, set_hook)
    local raw_load, raw_pcall = load, pcall
    local base_names = { 'assert', 'error', 'ipairs', 'next', 'pairs', 'select', 'tonumber', 'tostring', 'type', 'rawequal', 'rawget', 'rawset', 'rawlen', 'setmetatable' }
    local originals = {}
    for _, name in ipairs(base_names) do
        originals[name] = _G[name]
    end
    local libraries = { math = math, string = string, table = table, utf8 = utf8 }
    local callbacks = { 'activate', 'handle', 'render', 'overlay', 'service', 'deactivate' }
    return function(files_json, entry, state_json, name, args_json)
        local started, ticks, helper_bytes = clock(), 0, 0
        local function check()
            ticks = ticks + 1000
            assert(ticks <= 500000, 'plugin instruction budget exceeded')
            assert(clock() - started <= 250, 'plugin time budget exceeded')
        end
        local function execute()
            local sources = json.decode(files_json)
            local state = json.decode(state_json)
            assert(type(state) == 'table' and state ~= json.null, 'plugin state must be a JSON object')
            local env = { array = json.array, null = json.decode("null"), _VERSION = 'Lua 5.4' }
            for k, v in pairs(originals) do
                env[k] = v
            end
            for name, lib in pairs(libraries) do
                local copy = {};
                for k, v in pairs(lib) do
                    copy[k] = v
                end
                if name == 'string' then
                    copy.dump = nil
                end
                env[name] = copy
            end
            env._G = env
            local function helper(op, args)
                local input = json.encode(args)
                helper_bytes = helper_bytes + #input
                assert(helper_bytes <= 524288, 'native helper budget exceeded')
                local result = json.decode(host(op, input))
                assert(clock() - started <= 250, 'plugin time budget exceeded')
                assert(result.ok, result.error or "host helper failed")
                return result.value
            end
            env.graphemes = function(text)
                return helper('graphemes', { text = text })
            end
            env.join_text = function(a)
                return table.concat(a)
            end
            env.text_layout = function(text, size)
                return helper('text_layout', { text = text, size = size })
            end
            env.replace_text = function(s, needle, replacement)
                assert(type(s) == 'string' and type(needle) == 'string' and needle ~= '')
                local out, at = {}, 1
                while true do
                    local a, b = string.find(s, needle, at, true)
                    if not a then
                        out[#out + 1] = string.sub(s, at);
                        break
                    end
                    out[#out + 1] = string.sub(s, at, a - 1);
                    out[#out + 1] = replacement;
                    at = b + 1
                    assert(#out <= 65536, 'replacement limit')
                end
                return table.concat(out)
            end
            local loaded, loading, modules = {}, {}, 0
            local function require_local(path)
                assert(type(path) == 'string' and #path <= 256, 'invalid module path')
                assert(not path:find('\\', 1, true) and not path:find(':', 1, true) and path:sub(1, 1) ~= '/', 'absolute module path')
                if path:sub(-4) ~= '.lua' then
                    path = path .. '.lua'
                end
                for part in (path .. '/'):gmatch('(.-)/') do
                    assert(part ~= '' and part ~= '.' and part ~= '..', 'module traversal')
                end
                if loaded[path] ~= nil then
                    return loaded[path]
                end
                assert(not loading[path], 'module import cycle')
                modules = modules + 1;
                assert(modules <= 32, 'module count limit')
                local source = sources[path];
                assert(type(source) == 'string', 'module not found: ' .. path)
                local fn, err = raw_load(source, '@' .. path, 't', env);
                assert(fn, err)
                loading[path] = true
                local value = fn()
                assert(type(value) == 'table', 'a module must return a table: ' .. path)
                loading[path] = nil;
                loaded[path] = value;
                return value
            end
            env.require = require_local
            -- No io/os/package/debug/coroutine/load/dofile/loadfile/pcall/xpcall.
            local module = require_local(entry)
            local f = rawget(module, name);
            assert(type(f) == 'function', 'missing callback ' .. name)
            local args = json.decode(args_json)
            local result = f(state, table.unpack(args))
            local present = json.array()
            for _, key in ipairs(callbacks) do
                if type(rawget(module, key)) == 'function' then
                    present[#present + 1] = key
                end
            end
            return json.encode({ result = result == nil and json.null or result, state = state, callbacks = present })
        end
        set_hook(true, check)
        local ok, result = raw_pcall(execute)
        if not ok and type(result) ~= "string" then
            result = "plugin failed with a non-string error"
        end
        set_hook(false)
        if not ok then
            error(result, 0)
        end
        return result
    end
end
