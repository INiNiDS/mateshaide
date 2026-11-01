local J = {}
local array_mt = { __metatable = "array" }
local null_mt = { __metatable = "json.null" }
local function new_null()
    return setmetatable({}, null_mt)
end
J.null = new_null()
function J.array(...)
    return setmetatable({ ... }, array_mt)
end
local function escape(s)
    assert(utf8.len(s), "JSON string is not UTF-8")
    return '"' .. s:gsub('[%z\1-\31\\"]', function(c)
        local escapes = { ['"'] = '\\"', ['\\'] = '\\\\', ['\b'] = '\\b', ['\f'] = '\\f', ['\n'] = '\\n', ['\r'] = '\\r', ['\t'] = '\\t' }
        return escapes[c] or string.format('\\u%04x', c:byte())
    end) .. '"'
end
function J.encode(value)
    local active, pieces, bytes, count = {}, {}, 0, 0
    local function put(s)
        bytes = bytes + #s
        assert(bytes <= 1048576, "JSON output exceeds 1 MiB")
        pieces[#pieces + 1] = s
    end
    local emit
    emit = function(v, depth)
        assert(depth <= 32, "JSON nesting limit")
        count = count + 1
        assert(count <= 65536, "JSON value count limit")
        if v == J.null or v == nil or (type(v) == 'table' and getmetatable(v) == 'json.null') then
            put('null');
            return
        end
        local t = type(v)
        if t == 'boolean' then
            put(v and 'true' or 'false')
        elseif t == 'string' then
            put(escape(v))
        elseif t == 'number' then
            assert(v == v and v ~= math.huge and v ~= -math.huge, "non-finite JSON number")
            put(math.type(v) == 'integer' and string.format('%d', v) or string.format('%.17g', v))
        elseif t == 'table' then
            assert(not active[v], "cyclic JSON table")
            active[v] = true
            local n, numeric, strings = 0, false, false
            for k in next, v do
                if type(k) == 'number' and math.type(k) == 'integer' and k >= 1 then
                    numeric = true
                elseif type(k) == 'string' then
                    strings = true
                else
                    error('invalid JSON table key')
                end
                n = n + 1
            end
            local is_array = numeric or getmetatable(v) == "array"
            assert(not (numeric and strings), "mixed JSON table")
            if is_array then
                assert(not strings and rawlen(v) == n, "sparse JSON array")
                put('[')
                for i = 1, n do
                    if i > 1 then
                        put(',')
                    end ;
                    emit(rawget(v, i), depth + 1)
                end
                put(']')
            else
                put('{')
                local keys = {}
                for k in next, v do
                    keys[#keys + 1] = k
                end
                table.sort(keys)
                for i, k in ipairs(keys) do
                    if i > 1 then
                        put(',')
                    end
                    put(escape(k));
                    put(':');
                    emit(rawget(v, k), depth + 1)
                end
                put('}')
            end
            active[v] = nil
        else
            error('JSON cannot contain ' .. t)
        end
    end
    emit(value, 0)
    return table.concat(pieces)
end
function J.decode(s)
    assert(type(s) == 'string' and #s <= 16777216, 'JSON input limit')
    local pos, count = 1, 0
    local function whitespace()
        local _, finish = s:find('^[ \t\r\n]*', pos)
        pos = (finish or pos - 1) + 1
    end
    local function hex4()
        local h = s:sub(pos, pos + 3)
        assert(#h == 4 and h:match('^[0-9a-fA-F]+$'), 'bad JSON escape')
        pos = pos + 4;
        return tonumber(h, 16)
    end
    local function string_value()
        assert(s:sub(pos, pos) == '"');
        pos = pos + 1
        local out, begin = {}, pos
        while pos <= #s do
            local found = s:find('["\\%z\1-\31]', pos)
            assert(found, 'unterminated JSON string');
            pos = found
            local b = s:byte(pos)
            if b == 34 then
                out[#out + 1] = s:sub(begin, pos - 1);
                pos = pos + 1
                local value = table.concat(out);
                assert(utf8.len(value), 'bad UTF-8');
                return value
            elseif b == 92 then
                out[#out + 1] = s:sub(begin, pos - 1);
                pos = pos + 1
                local e = s:sub(pos, pos);
                pos = pos + 1
                local simple = { ['"'] = '"', ['\\'] = '\\', ['/'] = '/', b = '\b', f = '\f', n = '\n', r = '\r', t = '\t' }
                if e == 'u' then
                    local cp = hex4()
                    if cp >= 0xD800 and cp <= 0xDBFF then
                        assert(s:sub(pos, pos + 1) == '\\u', 'missing low surrogate');
                        pos = pos + 2
                        local low = hex4();
                        assert(low >= 0xDC00 and low <= 0xDFFF, 'bad low surrogate')
                        cp = 0x10000 + (cp - 0xD800) * 1024 + low - 0xDC00
                    else
                        assert(cp < 0xDC00 or cp > 0xDFFF, 'unpaired surrogate')
                    end
                    out[#out + 1] = utf8.char(cp)
                else
                    assert(simple[e], 'bad JSON escape');
                    out[#out + 1] = simple[e]
                end
                begin = pos
            else
                assert(b >= 32, 'JSON control character');
                pos = pos + 1
            end
        end
        error('unterminated JSON string')
    end
    local parse
    parse = function(depth)
        assert(depth <= 32, 'JSON depth limit');
        count = count + 1;
        assert(count <= 65536, 'JSON value count limit')
        whitespace();
        local c = s:sub(pos, pos)
        if c == '"' then
            return string_value()
        elseif c == '[' then
            pos = pos + 1;
            local a = J.array();
            whitespace()
            if s:sub(pos, pos) == ']' then
                pos = pos + 1;
                return a
            end
            while true do
                a[#a + 1] = parse(depth + 1);
                whitespace();
                c = s:sub(pos, pos);
                pos = pos + 1
                if c == ']' then
                    return a
                end ;
                assert(c == ',', 'expected JSON comma')
            end
        elseif c == '{' then
            pos = pos + 1;
            local o = {};
            whitespace()
            if s:sub(pos, pos) == '}' then
                pos = pos + 1;
                return o
            end
            while true do
                whitespace();
                local k = string_value();
                whitespace();
                assert(s:sub(pos, pos) == ':', 'expected JSON colon');
                pos = pos + 1
                assert(rawget(o, k) == nil, 'duplicate JSON key');
                o[k] = parse(depth + 1)
                whitespace();
                c = s:sub(pos, pos);
                pos = pos + 1
                if c == '}' then
                    return o
                end ;
                assert(c == ',', 'expected JSON comma')
            end
        elseif s:sub(pos, pos + 3) == 'true' then
            pos = pos + 4;
            return true
        elseif s:sub(pos, pos + 4) == 'false' then
            pos = pos + 5;
            return false
        elseif s:sub(pos, pos + 3) == 'null' then
            pos = pos + 4;
            return new_null()
        else
            local token = s:match('^-?%d+%.?%d*[eE]?[+-]?%d*', pos)
            assert(token and #token > 0, 'invalid JSON token')
            assert(not token:match('^%-?0%d') and not token:match('[%.eE+-]$'), 'bad JSON number')
            local n = tonumber(token);
            assert(n and n == n and n ~= math.huge and n ~= -math.huge, 'bad JSON number')
            pos = pos + #token;
            return n
        end
    end
    local value = parse(0);
    whitespace();
    assert(pos == #s + 1, 'trailing JSON data');
    return value
end
return J
