return function(json, host, clock, set_hook)
    local raw_load, raw_pcall, raw_dump = load, pcall, string.dump
    local base_names = { 'assert', 'error', 'ipairs', 'next', 'pairs', 'select', 'tonumber', 'tostring', 'type', 'rawequal', 'rawget', 'rawset', 'rawlen', 'setmetatable' }
    local originals = {}
    for _, name in ipairs(base_names) do
        originals[name] = _G[name]
    end
    local libraries = { math = math, string = string, table = table, utf8 = utf8 }
    local callbacks = { 'activate', 'handle', 'render', 'overlay', 'service', 'deactivate' }
    local cached_files, cached_sources, compiled = nil, nil, {}
    local function package_sources(files_json)
        if files_json ~= cached_files then
            cached_sources = json.decode(files_json)
            cached_files, compiled = files_json, {}
        end
        return cached_sources
    end
    local function compile(path, source, env)
        local bytecode = compiled[path]
        if not bytecode then
            local fn, err = raw_load(source, '@' .. path, 't', {})
            assert(fn, err)
            bytecode = raw_dump(fn)
            compiled[path] = bytecode
        end
        return raw_load(bytecode, '@' .. path, 'b', env)
    end
    local MAX_CACHED_HELPERS = 8192
    local helper_cache, helper_cached, cached_fonts = {}, 0, nil
    local function cache_helper(key, value)
        if helper_cached >= MAX_CACHED_HELPERS then
            helper_cache, helper_cached = {}, 0
        end
        helper_cache[key] = value
        helper_cached = helper_cached + 1
    end

    return function(files_json, entry, state_json, name, args_json)
        local started, ticks, helper_bytes, fonts_checked = clock(), 0, 0, false
        local function check()
            ticks = ticks + 1000
            assert(ticks <= 25000000, 'plugin instruction budget exceeded')
            assert(clock() - started <= 250, 'plugin time budget exceeded')
        end
        local function execute()
            local sources = package_sources(files_json)
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
            local function call_host(op, args)
                local input = json.encode(args)
                helper_bytes = helper_bytes + #input
                assert(helper_bytes <= 524288, 'native helper budget exceeded')
                local raw = host(op, input)
                assert(clock() - started <= 250, 'plugin time budget exceeded')
                return raw
            end
            local function helper(op, args)
                local result = json.decode(call_host(op, args))
                assert(result.ok, result.error or "host helper failed")
                return result.value
            end
            local NUMBERS_PREFIX = '{"ok":true,"value":['
            local function helper_numbers(op, args)
                if not fonts_checked then
                    fonts_checked = true
                    local generation = helper('font_generation', {})
                    if generation ~= cached_fonts then
                        helper_cache, helper_cached, cached_fonts = {}, 0, generation
                    end
                end
                local key = type(args.text) == 'string'
                        and op .. '\0' .. tostring(args.size) .. '\0' .. tostring(args.font) .. '\0' .. args.text
                local hit = key and helper_cache[key]
                if hit then
                    return hit, #hit
                end
                local raw = call_host(op, args)
                if raw:sub(1, #NUMBERS_PREFIX) ~= NUMBERS_PREFIX then
                    local result = json.decode(raw)
                    assert(result.ok, result.error or "host helper failed")
                    error('unexpected helper output')
                end
                local out, n = {}, 0
                for token in raw:gmatch('[^,%]}]+', #NUMBERS_PREFIX + 1) do
                    n = n + 1
                    out[n] = assert(tonumber(token), 'bad helper number')
                end
                if key then
                    cache_helper(key, out)
                end
                return out, n
            end
            local str_sub, to_int = string.sub, math.tointeger
            env.graphemes = function(text)
                local lengths, n = helper_numbers('grapheme_lengths', { text = text })
                local out, at = json.array(), 1
                for i = 1, n do
                    local len = lengths[i]
                    out[i] = str_sub(text, at, at + len - 1)
                    at = at + len
                end
                return out
            end
            env.join_text = function(a)
                return table.concat(a)
            end
            env.text_layout = function(text, size, font)
                local v, n = helper_numbers('text_layout_flat', { text = text, size = size, font = font })
                local lines, carets, k = json.array(), json.array(), 5
                for i = 1, to_int(v[4]) do
                    lines[i] = { start = to_int(v[k]), ['end'] = to_int(v[k + 1]), y = v[k + 2], width = v[k + 3] }
                    k = k + 4
                end
                local c = 0
                while k + 2 <= n do
                    c = c + 1
                    carets[c] = { x = v[k], y = v[k + 1], line = to_int(v[k + 2]) }
                    k = k + 3
                end
                return { width = v[1], height = v[2], line_height = v[3], carets = carets, lines = lines }
            end

            env.assets = {
                list = function()
                    return helper('asset_list', {})
                end,
                read = function(path)
                    return helper('asset_read', { path = path })
                end,
            }
            env.fonts = {
                load = function(path)
                    return helper('font_load', { path = path })
                end,
                list = function()
                    return helper('font_list', {})
                end,
            }
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
                local fn, err = compile(path, source, env);
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
            result = "plugin failed: " .. tostring(result)
        end
        set_hook(false)
        if not ok then
            error(result, 0)
        end
        return result
    end
end
