local E = {}

local floor, abs, exp, log, sqrt = math.floor, math.abs, math.exp, math.log, math.sqrt
local huge = math.huge

local function sign(v)
    if v > 0 then
        return 1
    elseif v < 0 then
        return -1
    end
    return 0
end

local function tanh(v)
    if v > 20 then
        return 1
    elseif v < -20 then
        return -1
    end
    local a = exp(v * 2)
    return (a - 1) / (a + 1)
end

local FUNCS = {
    sin = math.sin,
    cos = math.cos,
    tan = math.tan,
    cot = function(v)
        return 1 / math.tan(v)
    end,
    sec = function(v)
        return 1 / math.cos(v)
    end,
    csc = function(v)
        return 1 / math.sin(v)
    end,
    asin = math.asin,
    acos = math.acos,
    atan = function(v)
        return math.atan(v)
    end,
    sinh = function(v)
        return (exp(v) - exp(-v)) / 2
    end,
    cosh = function(v)
        return (exp(v) + exp(-v)) / 2
    end,
    tanh = tanh,
    ln = log,
    lg = function(v)
        return log(v, 10)
    end,
    log = function(v)
        return log(v, 10)
    end,
    sqrt = sqrt,
    cbrt = function(v)
        return v < 0 and -((-v) ^ (1 / 3)) or v ^ (1 / 3)
    end,
    abs = abs,
    exp = exp,
    floor = function(v)
        return floor(v) + 0.0
    end,
    ceil = function(v)
        return math.ceil(v) + 0.0
    end,
    round = function(v)
        return floor(v + 0.5) + 0.0
    end,
    sign = sign,
    sgn = sign,
    tg = math.tan,
    ctg = function(v)
        return 1 / math.tan(v)
    end,
}

local FUNCS2 = {
    min = math.min,
    max = math.max,
    mod = function(a, b)
        return a - b * floor(a / b)
    end,
}

local CONSTS = { pi = math.pi, e = exp(1) }

local NAMES = { "x", "y", "t" }
for k in pairs(FUNCS) do
    NAMES[#NAMES + 1] = k
end
for k in pairs(FUNCS2) do
    NAMES[#NAMES + 1] = k
end
for k in pairs(CONSTS) do
    NAMES[#NAMES + 1] = k
end
table.sort(NAMES, function(a, b)
    return #a > #b or (#a == #b and a < b)
end)

local REPLACE = {
    { "\\{", "(" },
    { "\\}", ")" },
    { "{", "(" },
    { "}", ")" },
    { "[", "(" },
    { "]", ")" },
    { "π", " pi " },
    { "·", "*" },
    { "×", "*" },
    { "⋅", "*" },
    { "−", "-" },
    { "÷", "/" },
    { "²", "^2" },
    { "³", "^3" },
    { "//", "/" },
    { "**", "^" },
}

local function replace_all(s, a, b)
    local out, at = {}, 1
    while true do
        local i, j = string.find(s, a, at, true)
        if not i then
            break
        end
        out[#out + 1] = string.sub(s, at, i - 1)
        out[#out + 1] = b
        at = j + 1
    end
    out[#out + 1] = string.sub(s, at)
    return table.concat(out)
end

function E.normalize(s)
    s = s:gsub("√([%w_%.]+)", "sqrt(%1)")
    s = replace_all(s, "√", "sqrt ")
    for _, r in ipairs(REPLACE) do
        s = replace_all(s, r[1], r[2])
    end
    return s
end


local SYMBOLS = "+-*/^(),|_"

local function read_number(s, i)
    local num = s:match("^%d*%.?%d*", i)
    local v = tonumber(num)
    if not v then
        return nil, nil, "bad number '" .. num .. "'"
    end
    return { t = "num", v = v + 0.0 }, #num
end

local function name_at(word, j)
    for _, name in ipairs(NAMES) do
        if word:sub(j, j + #name - 1) == name then
            return name
        end
    end
end

local function split_word(word)
    local names, j = {}, 1
    while j <= #word do
        local name = name_at(word, j)
        if not name then
            return nil, "unknown name '" .. word:sub(j) .. "'"
        end
        names[#names + 1] = name
        j = j + #name
    end
    return names
end

local CHAR_CLASSES = {
    { "space", "%s" },
    { "number", "[%d%.]" },
    { "word", "%a" },
}

local function classify(c)
    for _, cls in ipairs(CHAR_CLASSES) do
        if c:match(cls[2]) then
            return cls[1]
        end
    end
    if SYMBOLS:find(c, 1, true) then
        return "symbol"
    end
end

local READERS = {
    space = function()
        return {}, 1
    end,
    number = function(s, i)
        local tok, len, err = read_number(s, i)
        if not tok then
            return nil, nil, err
        end
        return { tok }, len
    end,
    word = function(s, i)
        local word = s:match("^%a+", i)
        local names, err = split_word(word)
        if not names then
            return nil, nil, err
        end
        local toks = {}
        for _, name in ipairs(names) do
            toks[#toks + 1] = { t = "id", v = name }
        end
        return toks, #word
    end,
    symbol = function(s, i)
        return { { t = s:sub(i, i) } }, 1
    end,
}

local function tokenize(s)
    local toks, i = {}, 1
    while i <= #s do
        local kind = classify(s:sub(i, i))
        if not kind then
            return nil, "unexpected '" .. (s:match("^" .. utf8.charpattern, i) or s:sub(i, i)) .. "'"
        end
        local new, len, err = READERS[kind](s, i)
        if not new then
            return nil, err
        end
        for _, tok in ipairs(new) do
            toks[#toks + 1] = tok
        end
        i = i + len
    end
    return toks
end


local function const(v)
    return {
        c = v,
        f = function()
            return v
        end,
    }
end

local function identity(v)
    return v
end

local function variable()
    return { f = identity, var = true }
end

local function apply1(fn, a)
    if a.c then
        return const(fn(a.c))
    end
    local fa = a.f
    return {
        f = function(v)
            return fn(fa(v))
        end,
    }
end

local function apply2(fn, a, b)
    if a.c and b.c then
        return const(fn(a.c, b.c))
    end
    local fa, fb = a.f, b.f
    return {
        f = function(v)
            return fn(fa(v), fb(v))
        end,
    }
end

local function add(x, y)
    return x + y
end
local function sub(x, y)
    return x - y
end
local function mul(x, y)
    return x * y
end
local function div(x, y)
    return x / y
end
local function neg(x)
    return -x
end

local function log_with_base(a, base)
    return apply2(function(x, b)
        return log(x) / log(b)
    end, a, base)
end

local function power(a, b)
    if a.c and b.c then
        return const(a.c ^ b.c)
    end
    if not b.c then
        return apply2(function(x, y)
            return x ^ y
        end, a, b)
    end
    local c, fa = b.c, a.f
    if c == 2 then
        return {
            f = function(v)
                local u = fa(v)
                return u * u
            end,
        }
    end
    if c == floor(c) then
        return {
            f = function(v)
                return fa(v) ^ c
            end,
        }
    end
    for q = 3, 9, 2 do
        local p = c * q
        if abs(p - floor(p + 0.5)) < 1e-9 then
            local odd = floor(p + 0.5) % 2 ~= 0
            return {
                f = function(v)
                    local u = fa(v)
                    if u >= 0 then
                        return u ^ c
                    end
                    local r = (-u) ^ c
                    return odd and -r or r
                end,
            }
        end
    end
    return {
        f = function(v)
            return fa(v) ^ c
        end,
    }
end

local function describe(k)
    if not k then
        return "end"
    end
    return k.v and tostring(k.v) or k.t
end

local ADD_OPS = { ["+"] = add, ["-"] = sub }
local MUL_OPS = { ["*"] = mul, ["/"] = div }

local Parser = {}
Parser.__index = Parser

local function is_func(k)
    return k and k.t == "id" and (FUNCS[k.v] or FUNCS2[k.v]) ~= nil
end

local function new_parser(toks, vars)
    return setmetatable({ toks = toks, vars = vars, pos = 1, bars = 0, err = nil }, Parser)
end

function Parser:fail(msg)
    self.err = self.err or msg
    return nil
end

function Parser:peek()
    return self.toks[self.pos]
end

function Parser:at(t)
    local k = self:peek()
    return k ~= nil and k.t == t
end

function Parser:expect(t, msg)
    if not self:at(t) then
        return self:fail(msg)
    end
    self.pos = self.pos + 1
    return true
end

function Parser:starts_operand()
    local k = self:peek()
    if not k then
        return false
    end
    if k.t == "|" then
        return self.bars == 0
    end
    return k.t == "num" or k.t == "id" or k.t == "("
end

-- ( expr )
function Parser:group()
    if not self:at("(") then
        return self:fail("expected '('")
    end
    self.pos = self.pos + 1
    local e = self:expr()
    if not e or not self:expect(")", "expected ')'") then
        return nil
    end
    return e
end

function Parser:bare_arg()
    local a = self:pow()
    while a and self:starts_operand() and not is_func(self:peek()) do
        local b = self:pow()
        if not b then
            return nil
        end
        a = apply2(mul, a, b)
    end
    return a
end

function Parser:log_base(name)
    if name ~= "log" or not self:at("_") then
        return nil, true
    end
    self.pos = self.pos + 1
    local k = self:peek()
    if k and k.t == "num" then
        self.pos = self.pos + 1
        return const(k.v), true
    end
    local b = self:group()
    return b, b ~= nil
end

function Parser:func_exponent()
    if not self:at("^") then
        return nil, true
    end
    self.pos = self.pos + 1
    local e = self:unary()
    return e, e ~= nil
end

function Parser:two_args(name)
    if not self:at("(") then
        return self:fail(name .. " needs (a, b)")
    end
    self.pos = self.pos + 1
    local a = self:expr()
    if not a or not self:expect(",", name .. " needs two arguments") then
        return nil
    end
    local b = self:expr()
    if not b or not self:expect(")", "expected ')'") then
        return nil
    end
    return apply2(FUNCS2[name], a, b)
end

-- f(x) или f x
function Parser:one_arg(name)
    if not self:peek() then
        return self:fail("missing argument of " .. name)
    end
    local a
    if self:at("(") then
        a = self:group()
    else
        a = self:bare_arg()
    end
    if not a then
        return self:fail("missing argument of " .. name)
    end
    return a
end

function Parser:func_value(name, base)
    if FUNCS2[name] then
        return self:two_args(name)
    end
    local a = self:one_arg(name)
    if not a then
        return nil
    end
    if base then
        return log_with_base(a, base)
    end
    return apply1(FUNCS[name], a)
end

function Parser:call(name)
    local base, base_ok = self:log_base(name)
    if not base_ok then
        return nil
    end
    local exponent, exponent_ok = self:func_exponent()
    if not exponent_ok then
        return nil
    end
    local r = self:func_value(name, base)
    if not r then
        return nil
    end
    if exponent then
        r = power(r, exponent)
    end
    return r
end

-- |expr|
function Parser:parse_abs()
    self.pos = self.pos + 1
    self.bars = self.bars + 1
    local e = self:expr()
    if not e or not self:expect("|", "expected '|'") then
        return nil
    end
    self.bars = self.bars - 1
    return apply1(abs, e)
end

function Parser:parse_id(name)
    if CONSTS[name] then
        return const(CONSTS[name])
    end
    if FUNCS[name] or FUNCS2[name] then
        return self:call(name)
    end
    if not self.vars[name] then
        return self:fail("variable " .. name .. " is not allowed here")
    end
    return variable()
end

local PRIMARY = {
    num = function(p, k)
        p.pos = p.pos + 1
        return const(k.v)
    end,
    id = function(p, k)
        p.pos = p.pos + 1
        return p:parse_id(k.v)
    end,
    ["("] = function(p)
        return p:group()
    end,
    ["|"] = function(p)
        return p:parse_abs()
    end,
}

function Parser:primary()
    local k = self:peek()
    if not k then
        return self:fail("unexpected end")
    end
    local handler = PRIMARY[k.t]
    if not handler then
        return self:fail("unexpected '" .. describe(k) .. "'")
    end
    return handler(self, k)
end

function Parser:pow()
    local a = self:primary()
    if not a then
        return nil
    end
    if not self:at("^") then
        return a
    end
    self.pos = self.pos + 1
    local b = self:unary()
    if not b then
        return nil
    end
    return power(a, b)
end

function Parser:unary()
    if self:at("-") then
        self.pos = self.pos + 1
        local a = self:unary()
        return a and apply1(neg, a)
    elseif self:at("+") then
        self.pos = self.pos + 1
        return self:unary()
    end
    return self:pow()
end

function Parser:term()
    local a = self:unary()
    while a do
        local k = self:peek()
        if k and MUL_OPS[k.t] then
            self.pos = self.pos + 1
            local b = self:unary()
            if not b then
                return nil
            end
            a = apply2(MUL_OPS[k.t], a, b)
        elseif self:starts_operand() then
            local b = self:pow()
            if not b then
                return nil
            end
            a = apply2(mul, a, b)
        else
            break
        end
    end
    return a
end

function Parser:expr()
    local a = self:term()
    while a do
        local k = self:peek()
        if not (k and ADD_OPS[k.t]) then
            break
        end
        self.pos = self.pos + 1
        local b = self:term()
        if not b then
            return nil
        end
        a = apply2(ADD_OPS[k.t], a, b)
    end
    return a
end

local function parse(toks, vars)
    if #toks == 0 then
        return nil, "empty formula"
    end
    local p = new_parser(toks, vars)
    local e = p:expr()
    if e and p.pos <= #toks then
        p:fail("unexpected '" .. describe(toks[p.pos]) .. "'")
    end
    if not e or p.err then
        return nil, p.err or "syntax error"
    end
    return e
end

function E.compile(text, var)
    local toks, err = tokenize(E.normalize(text))
    if not toks then
        return nil, err
    end
    local node, perr = parse(toks, var and { [var] = true } or {})
    if not node then
        return nil, perr
    end
    return node.f, node.c
end

local function split_top_comma(s)
    local depth = 0
    for i = 1, #s do
        local c = s:sub(i, i)
        if c == "(" or c == "{" or c == "[" then
            depth = depth + 1
        elseif c == ")" or c == "}" or c == "]" then
            depth = depth - 1
        elseif c == "," and depth == 0 then
            return s:sub(1, i - 1), s:sub(i + 1)
        end
    end
end

local function finite(v)
    return v == v and v ~= huge and v ~= -huge
end

local function split_t_range(s)
    local body, a, b = s:match("^(.-)[;,]%s*t%s*=%s*([^;]-)%.%.([^;]+)$")
    if not body then
        return { s = s, t0 = 0, t1 = 2 * math.pi, ranged = false }
    end
    local _, ca = E.compile(a, nil)
    local _, cb = E.compile(b, nil)
    if not (ca and cb and finite(ca) and finite(cb) and ca < cb) then
        return nil, "t range must be numbers a..b with a < b"
    end
    return { s = body, t0 = ca, t1 = cb, ranged = true }
end

local function parse_parametric(r)
    local inner = r.s:match("^%s*%((.*)%)%s*$")
    local px, py
    if inner then
        px, py = split_top_comma(inner)
    end
    if not px then
        px, py = r.s:match("^%s*x%s*=(.-);%s*y%s*=(.+)$")
    end
    if not px then
        return nil
    end
    local fx, ex = E.compile(px, "t")
    if not fx then
        return { error = "x(t): " .. ex }
    end
    local fy, ey = E.compile(py, "t")
    if not fy then
        return { error = "y(t): " .. ey }
    end
    return { kind = "param", fx = fx, fy = fy, t0 = r.t0, t1 = r.t1 }
end

local function parse_explicit(s)
    local rhs = s:match("^%s*y%s*=(.*)$") or s:match("^%s*%a%s*%(%s*x%s*%)%s*=(.*)$")
    if rhs then
        local f, e = E.compile(rhs, "x")
        if not f then
            return { error = e }
        end
        return { kind = "fx", f = f }
    end
    rhs = s:match("^%s*x%s*=(.*)$")
    if rhs then
        local f, e = E.compile(rhs, "y")
        if not f then
            return { error = e }
        end
        return { kind = "xy", f = f }
    end
    return nil
end

function E.row(text)
    if type(text) ~= "string" or not text:find("%S") then
        return nil
    end
    local r, err = split_t_range(E.normalize(text))
    if not r then
        return { error = err }
    end

    local par = parse_parametric(r)
    if par then
        return par
    end
    if r.ranged then
        return { error = "t = a..b only goes with (x(t), y(t))" }
    end

    local explicit = parse_explicit(r.s)
    if explicit then
        return explicit
    end
    if r.s:find("=", 1, true) then
        return { error = "use y = …, x = … or (x(t), y(t))" }
    end

    local f, e = E.compile(r.s, "x")
    if not f then
        return { error = e }
    end
    return { kind = "fx", f = f }
end

return E