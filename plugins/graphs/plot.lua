local Plot = {}

local floor, ceil, abs, min, max = math.floor, math.ceil, math.abs, math.min, math.max
local log, huge = math.log, math.huge

Plot.PAD, Plot.ROW, Plot.FONT, Plot.MIN_PLOT = 8, 18, 12, 110
Plot.PALETTE = {
    { 0.18, 0.40, 0.88, 1.0 },
    { 0.85, 0.20, 0.20, 1.0 },
    { 0.10, 0.55, 0.30, 1.0 },
    { 0.58, 0.28, 0.78, 1.0 },
    { 0.93, 0.52, 0.08, 1.0 },
    { 0.05, 0.60, 0.66, 1.0 },
}
local GRID = { 0.50, 0.52, 0.58, 0.22 }
local GRID_MINOR = { 0.50, 0.52, 0.58, 0.09 }
local AXIS = { 0.20, 0.22, 0.28, 0.90 }
local LABEL = { 0.30, 0.32, 0.38, 1.0 }
local FRAME = { 0.45, 0.47, 0.55, 0.45 }
local BACK = { 0.50, 0.52, 0.60, 0.06 }

local MAX_DEPTH = 7
local TICK_FONT = 10
local CURVE_WIDTH = 2.2

local function finite(v) return v == v and v ~= huge and v ~= -huge end
Plot.finite = finite

local function q(v) return floor(v * 16 + 0.5) / 16 end

function Plot.layout(node)
    local n = max(1, #node.data.rows)
    local head = Plot.PAD + n * Plot.ROW + 4
    return {
        rows_top = node.y - Plot.PAD,
        text_x = node.x + Plot.PAD + 18,
        head = head,
        l = node.x + Plot.PAD,
        r = node.x + node.width - Plot.PAD,
        t = node.y - head,
        b = node.y - node.height + Plot.PAD,
    }
end

function Plot.min_height(rows)
    return Plot.PAD + max(1, rows) * Plot.ROW + 4 + Plot.MIN_PLOT + Plot.PAD
end

function Plot.nice_step(span, count)
    local raw = span / max(1, count)
    local p = 10 ^ floor(log(raw, 10))
    local m = raw / p
    local k = m < 1.5 and 1 or m < 3.5 and 2 or m < 7.5 and 5 or 10
    return k * p
end

function Plot.label(v, step)
    local s
    if step >= 1e5 or step < 1e-4 then
        s = string.format("%.3g", v)
    else
        local d = max(0, -floor(log(step, 10) + 1e-9))
        s = string.format("%." .. d .. "f", v)
    end
    if s:match("^%-0%.?0*$") then s = s:sub(2) end
    return s
end

local function line(out, x1, y1, x2, y2, w, c)
    out[#out + 1] = { kind = "line", x1 = q(x1), y1 = q(y1), x2 = q(x2), y2 = q(y2), width = w, color = c }
end

local function text(out, x, y, s, size, c)
    out[#out + 1] = { kind = "text", x = q(x), y = q(y), text = s, size = size, color = c }
end

local function rect(out, x, y, w, h, c)
    out[#out + 1] = { kind = "rect", x = q(x), y = q(y), width = q(w), height = q(h), color = c }
end
Plot.rect, Plot.text, Plot.line = rect, text, line

local function grid(lo, hi, step, scale, draw)
    for _, g in ipairs({ { step / 5, GRID_MINOR }, { step, GRID } }) do
        local s = g[1]
        if s * scale >= 10 then
            for k = ceil(lo / s), floor(hi / s) do
                draw(k * s, g[2])
            end
        end
    end
end

local function ticks(lo, hi, step)
    local res = {}
    for k = ceil(lo / step), floor(hi / step) do
        local v = k * step
        if abs(v) > step * 1e-6 then res[#res + 1] = v end
    end
    return res
end

function Plot.axes(out, view, L, grid_on)
    local x0, x1, y0, y1 = view[1], view[2], view[3], view[4]
    local l, r, b, t = L.l, L.r, L.b, L.t
    rect(out, l, t, r - l, t - b, BACK)
    local sx, sy = (r - l) / (x1 - x0), (t - b) / (y1 - y0)
    local function X(v) return l + (v - x0) * sx end
    local function Y(v) return b + (v - y0) * sy end
    local xs = Plot.nice_step(x1 - x0, (r - l) / 70)
    local ys = Plot.nice_step(y1 - y0, (t - b) / 50)

    if grid_on then
        grid(x0, x1, xs, sx, function(v, c) line(out, X(v), b, X(v), t, 1.0, c) end)
        grid(y0, y1, ys, sy, function(v, c) line(out, l, Y(v), r, Y(v), 1.0, c) end)
    end

    local ax = min(max(Y(0), b), t)
    local ay = min(max(X(0), l), r)
    if y0 <= 0 and y1 >= 0 then line(out, l, ax, r, ax, 1.6, AXIS) end
    if x0 <= 0 and x1 >= 0 then line(out, ay, b, ay, t, 1.6, AXIS) end

    local below = ax - 3
    if below - TICK_FONT * 1.25 < b then below = ax + TICK_FONT * 1.25 + 2 end
    for _, v in ipairs(ticks(x0, x1, xs)) do
        local s = Plot.label(v, xs)
        local w = #s * TICK_FONT * 0.6
        local x = min(max(X(v) - w / 2, l + 1), r - w - 1)
        text(out, x, below, s, TICK_FONT, LABEL)
    end

    local right = ay + 4 + 30 > r
    for _, v in ipairs(ticks(y0, y1, ys)) do
        local s = Plot.label(v, ys)
        local w = #s * TICK_FONT * 0.6
        local x = ay - 4 - w
        if not right and x < l then x = ay + 4 end
        local y = min(max(Y(v) + TICK_FONT * 0.62, b + TICK_FONT * 1.25), t - 1)
        text(out, x, y, s, TICK_FONT, LABEL)
    end

    if x0 <= 0 and x1 >= 0 and y0 <= 0 and y1 >= 0 then
        text(out, X(0) - 10, Y(0) - 3, "0", TICK_FONT, LABEL)
    end
    line(out, l, t, r, t, 1.0, FRAME)
    line(out, l, b, r, b, 1.0, FRAME)
    line(out, l, b, l, t, 1.0, FRAME)
    line(out, r, b, r, t, 1.0, FRAME)
end

function Plot.clip(x1, y1, x2, y2, l, r, b, t)
    local dx, dy = x2 - x1, y2 - y1
    local p = { -dx, dx, -dy, dy }
    local q = { x1 - l, r - x1, y1 - b, t - y1 }
    local u0, u1 = 0, 1

    for i = 1, 4 do
        if p[i] == 0 then
            if q[i] < 0 then return end
        else
            local u = q[i] / p[i]
            if p[i] < 0 then u0 = max(u0, u) else u1 = min(u1, u) end
            if u0 > u1 then return end
        end
    end

    return x1 + u0 * dx, y1 + u0 * dy, x1 + u1 * dx, y1 + u1 * dy
end

local function new_builder()
    local sb = { runs = {}, run = nil, count = 0 }

    function sb.add(x, y)
        if not sb.run then
            sb.run = {}
            sb.runs[#sb.runs + 1] = sb.run
        end
        local run = sb.run
        run[#run + 1] = x
        run[#run + 1] = y
        sb.count = sb.count + 1
    end

    function sb.break_run()
        sb.run = nil
    end

    return sb
end

local function outside(ax, ay, bx, by, mx, my, L)
    local l, r, b, t = L.l, L.r, L.b, L.t
    return (ay > t and by > t and my > t) or (ay < b and by < b and my < b)
            or (ax > r and bx > r and mx > r) or (ax < l and bx < l and mx < l)
end

local function is_jump(ax, ay, bx, by, mx, my, jump2)
    local cx, cy = bx - ax, by - ay
    local len2 = cx * cx + cy * cy
    if len2 <= jump2 then return false end
    local u = ((mx - ax) * cx + (my - ay) * cy) / len2
    return u < 0.1 or u > 0.9
end

local function needs_split(ax, ay, bx, by, mx, my, tol2, L)
    local fa, fb, fm = ax ~= nil, bx ~= nil, mx ~= nil
    if fa ~= fb or fa ~= fm then return true end
    if not fa then return false end
    if outside(ax, ay, bx, by, mx, my, L) then return false end
    local dx, dy = mx - (ax + bx) * 0.5, my - (ay + by) * 0.5
    return dx * dx + dy * dy > tol2
end

local function emit(sb, ax, ay, bx, by, mx, my, ctx)
    local defined = ax ~= nil and bx ~= nil and mx ~= nil
    local off = defined and outside(ax, ay, bx, by, mx, my, ctx.L)

    if off or (defined and not is_jump(ax, ay, bx, by, mx, my, ctx.jump2)) then
        if not sb.run then sb.add(ax, ay) end
        sb.add(bx, by)
    else
        sb.break_run()
        if bx then sb.add(bx, by) end
    end
end

local function seg(ctx, s, ax, ay, u, bx, by, depth)
    local m = (s + u) * 0.5
    local mx, my = ctx.point(m)

    if depth < MAX_DEPTH and ctx.sb.count < ctx.cap
            and needs_split(ax, ay, bx, by, mx, my, ctx.tol2, ctx.L) then
        seg(ctx, s, ax, ay, m, mx, my, depth + 1)
        seg(ctx, m, mx, my, u, bx, by, depth + 1)
        return
    end

    emit(ctx.sb, ax, ay, bx, by, mx, my, ctx)
end

function Plot.trace(point, s0, s1, n, tol, L, cap)
    local ctx = {
        point = point,
        L = L,
        cap = cap,
        tol2 = tol * tol,
        jump2 = (4 * tol) ^ 2,
        sb = new_builder(),
    }

    local ps, px, py = s0, point(s0)
    for i = 1, n do
        local u = s0 + (s1 - s0) * i / n
        local ux, uy = point(u)
        seg(ctx, ps, px, py, u, ux, uy, 0)
        ps, px, py = u, ux, uy
    end
    return ctx.sb.runs
end

function Plot.simplify(run, tol)
    local n = #run // 2
    if n <= 2 then return run end
    local out = { run[1], run[2] }
    local i = 1
    while i < n do
        local j = i + 1
        while j < n do
            local ax, ay = run[2 * i - 1], run[2 * i]
            local cx, cy = run[2 * j + 1] - ax, run[2 * j + 2] - ay
            local len = math.sqrt(cx * cx + cy * cy)
            local ok = len > 0
            for k = i + 1, j do
                if not ok then break end
                local d = abs((run[2 * k - 1] - ax) * cy - (run[2 * k] - ay) * cx) / len
                ok = d <= tol
            end
            if not ok then break end
            j = j + 1
        end
        out[#out + 1] = run[2 * j - 1]
        out[#out + 1] = run[2 * j]
        i = j
    end
    return out
end

local function make_world(view, L)
    local x0, y0 = view[1], view[3]
    local l, b = L.l, L.b
    local sx = (L.r - l) / (view[2] - x0)
    local sy = (L.t - b) / (view[4] - y0)
    return function(x, y)
        local X, Y = l + (x - x0) * sx, b + (y - y0) * sy
        if finite(X) and finite(Y) then return X, Y end
    end
end

local function curve_source(fn, world, view)
    if fn.kind == "fx" then
        local f = fn.f
        return function(s) return world(s, f(s)) end, view[1], view[2]
    elseif fn.kind == "xy" then
        local f = fn.f
        return function(s) return world(f(s), s) end, view[3], view[4]
    end
    local fx, fy = fn.fx, fn.fy
    return function(s) return world(fx(s), fy(s)) end, fn.t0, fn.t1
end

local function sample_count(fn, L, zoom)
    if fn.kind == "param" then return 200 end
    local span = fn.kind == "xy" and (L.t - L.b) or (L.r - L.l)
    local px = max(1, span * zoom)
    return min(400, max(24, floor(px / 5)))
end

local function in_rect(x, y, L)
    return x >= L.l and x <= L.r and y >= L.b and y <= L.t
end

local function draw_segment(out, ax, ay, bx, by, L, color)
    if in_rect(ax, ay, L) and in_rect(bx, by, L) then
        line(out, ax, ay, bx, by, CURVE_WIDTH, color)
        return
    end
    local cx1, cy1, cx2, cy2 = Plot.clip(ax, ay, bx, by, L.l, L.r, L.b, L.t)
    if cx1 then line(out, cx1, cy1, cx2, cy2, CURVE_WIDTH, color) end
end

local function draw_run(out, run, L, color, zoom)
    run = Plot.simplify(run, 0.3 / zoom)
    for i = 1, #run - 3, 2 do
        draw_segment(out, run[i], run[i + 1], run[i + 2], run[i + 3], L, color)
    end
end

function Plot.curve(out, fn, view, L, zoom, color, cap)
    local world = make_world(view, L)
    local point, s0, s1 = curve_source(fn, world, view)
    local n = sample_count(fn, L, zoom)
    local runs = Plot.trace(point, s0, s1, n, 0.35 / zoom, L, cap)
    for _, run in ipairs(runs) do
        draw_run(out, run, L, color, zoom)
    end
end

local function samples(fn, view)
    local xs, ys = {}, {}
    local function push(x, y)
        if finite(x) and finite(y) then
            xs[#xs + 1] = x
            ys[#ys + 1] = y
        end
    end
    local n = 240
    for i = 0, n do
        if fn.kind == "fx" then
            local x = view[1] + (view[2] - view[1]) * i / n
            push(x, fn.f(x))
        elseif fn.kind == "xy" then
            local y = view[3] + (view[4] - view[3]) * i / n
            push(fn.f(y), y)
        else
            local s = fn.t0 + (fn.t1 - fn.t0) * i / n
            push(fn.fx(s), fn.fy(s))
        end
    end
    return xs, ys
end

local function robust(v)
    table.sort(v)
    local n = #v
    local lo, hi = v[floor(n * 0.05) + 1], v[max(1, ceil(n * 0.95))]
    local span = hi - lo
    lo, hi = max(v[1], lo - span * 0.5), min(v[n], hi + span * 0.5)
    if hi - lo < 1e-9 then
        lo, hi = lo - 1, hi + 1
    end
    local pad = (hi - lo) * 0.08
    return lo - pad, hi + pad
end

local function append(dst, src)
    for _, v in ipairs(src) do dst[#dst + 1] = v end
end

local function collect(fns, view)
    local ax, ay = {}, {}
    local explicit, curves = false, false
    for _, fn in ipairs(fns) do
        local xs, ys = samples(fn, view)
        if fn.kind == "fx" then
            explicit = true
            append(ay, ys)
        else
            curves = true
            append(ax, xs)
            if fn.kind == "param" then append(ay, ys) end
        end
    end
    return ax, ay, explicit, curves
end

local function square_view(x0, x1, y0, y1, L)
    local w, h = max(1, L.r - L.l), max(1, L.t - L.b)
    local u = max((x1 - x0) / w, (y1 - y0) / h)
    local cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    return cx - u * w / 2, cx + u * w / 2, cy - u * h / 2, cy + u * h / 2
end

function Plot.fit(fns, view, L)
    local ax, ay, explicit, curves = collect(fns, view)
    local x0, x1, y0, y1 = view[1], view[2], view[3], view[4]
    if #ay > 0 then y0, y1 = robust(ay) end
    if not explicit and #ax > 0 then x0, x1 = robust(ax) end
    if curves and not explicit then
        x0, x1, y0, y1 = square_view(x0, x1, y0, y1, L)
    end
    return { x0, x1, y0, y1 }
end

function Plot.zoom(view, k, cx, cy)
    local x0, x1, y0, y1 = view[1], view[2], view[3], view[4]
    cx, cy = cx or (x0 + x1) / 2, cy or (y0 + y1) / 2
    local sx, sy = (x1 - x0) * k, (y1 - y0) * k
    if sx < 1e-6 or sy < 1e-6 or sx > 1e8 or sy > 1e8 then return view end
    return {
        cx + (x0 - cx) * k, cx + (x1 - cx) * k,
        cy + (y0 - cy) * k, cy + (y1 - cy) * k,
    }
end

return Plot