local Expr = require("expr")
local Plot = require("plot")

local P = {}
local min, max, abs, floor, exp = math.min, math.max, math.abs, math.floor, math.exp

local DEFAULT_W, DEFAULT_H = 480, 340
local MIN_W = 200
local HANDLE = 8
local BAR = 12
local MAX_ROWS = 12
local HINT = "y = f(x),  x = g(y),  (x(t), y(t)); t = a..b"
local ERROR_C = { 0.85, 0.16, 0.16, 1.0 }
local TEXT_C = { 0.16, 0.18, 0.24, 1.0 }

local function find_node(nodes, id)
    for _, n in ipairs(nodes) do
        if n.id == id then
            return n
        end
    end
end

local function copy4(v)
    return { v[1], v[2], v[3], v[4] }
end

local function update(node)
    return {
        kind = "node.update",
        node = node,
    }
end

local function rows_of(node)
    local out = array()
    for i, row in ipairs(node.data.rows) do
        out[i] = row
    end
    return out
end

local function color_key(c)
    return string.format("%.3f %.3f %.3f", c[1], c[2], c[3])
end

local function next_color(rows)
    local used = {}
    for _, row in ipairs(rows) do
        used[color_key(row.color)] = true
    end
    for _, c in ipairs(Plot.PALETTE) do
        if not used[color_key(c)] then
            return copy4(c)
        end
    end
    return copy4(Plot.PALETTE[#rows % #Plot.PALETTE + 1])
end

local function compile_rows(rows)
    local fns = {}
    for i, row in ipairs(rows) do
        fns[i] = Expr.row(row.text) or false
    end
    return fns
end

local function valid(fns)
    local out = {}
    for _, fn in ipairs(fns) do
        if fn and not fn.error then
            out[#out + 1] = fn
        end
    end
    return out
end

local function refit(node)
    node.height = max(node.height, Plot.min_height(#node.data.rows))
    if node.data.auto then
        local fns = valid(compile_rows(node.data.rows))
        if #fns > 0 then
            node.data.view = Plot.fit(fns, node.data.view, Plot.layout(node))
        end
    end
    return node
end

local function new_graph(ctx, x, y, texts)
    local rows = array()
    for i, text in ipairs(texts) do
        rows[i] = { text = text, color = copy4(Plot.PALETTE[(i - 1) % #Plot.PALETTE + 1]) }
    end
    local node = {
        id = ctx.new_id,
        type = "graph",
        schema_version = 1,
        x = x,
        y = y,
        width = DEFAULT_W,
        height = DEFAULT_H,
        data = { rows = rows, view = { -10, 10, -6.5, 6.5 }, auto = true, grid = true },
    }
    return refit(node)
end

local function corners(n)
    return {
        { x = n.x, y = n.y },
        { x = n.x + n.width, y = n.y },
        { x = n.x + n.width, y = n.y - n.height },
        { x = n.x, y = n.y - n.height },
    }
end

local function hit_corner(n, x, y)
    for i, c in ipairs(corners(n)) do
        if abs(x - c.x) <= HANDLE and abs(y - c.y) <= HANDLE then
            return i
        end
    end
end

local function on_bar(n, x, y)
    return x >= n.x - 4 and x <= n.x + n.width + 4 and y <= n.y + BAR and y >= n.y - 2
end

local function find_hit(nodes, x, y)
    for i = #nodes, 1, -1 do
        local n = nodes[i]
        if n.type == "graph" and x >= n.x and x <= n.x + n.width and y <= n.y and y >= n.y - n.height then
            return n
        end
    end
end

local function row_at(node, y)
    local L = Plot.layout(node)
    if y < L.t then
        return nil
    end
    return min(#node.data.rows, max(1, floor((L.rows_top - y) / Plot.ROW) + 1))
end

local function caret_x(text, i)
    if text == "" or i <= 0 then
        return 0
    end
    local l = text_layout(text, Plot.FONT)
    local c = l.carets[i + 1] or l.carets[#l.carets]
    return c and c.x or 0
end

local function caret_at(node, row, x)
    local text = node.data.rows[row].text
    if text == "" then
        return 0
    end
    local carets = text_layout(text, Plot.FONT).carets
    local L, best, dist = Plot.layout(node), 0, math.huge
    for i, c in ipairs(carets) do
        local d = abs(L.text_x + c.x - x)
        if d < dist then
            best, dist = i - 1, d
        end
    end
    return best
end

local function cleanup(node)
    if not node then
        return array()
    end
    local rows = array()
    for _, row in ipairs(node.data.rows) do
        if row.text:find("%S") then
            rows[#rows + 1] = row
        end
    end
    if #rows == 0 then
        return { { kind = "node.delete", id = node.id } }
    end
    if #rows == #node.data.rows then
        return array()
    end
    node.data.rows = rows
    return { update(refit(node)) }
end

local function finish(s, ctx)
    local out = cleanup(find_node(ctx.nodes, s.current))
    s.row, s.drag, s.fresh = 0, "", false
    out[#out + 1] = { kind = "input.blur" }
    return out
end

local function set_text(s, node, text)
    node.data.rows[s.row].text = text
    return { update(refit(node)) }
end

local function insert_text(s, node, text)
    text = text:gsub("[\r\n\t]", " ")
    local row = node.data.rows[s.row]
    if s.fresh then
        s.fresh, s.caret = false, #graphemes(text)
        return set_text(s, node, text)
    end
    local gs = graphemes(row.text)
    local at = min(s.caret, #gs)
    local before = table.concat(gs, "", 1, at)
    local after = table.concat(gs, "", at + 1, #gs)
    s.caret = at + #graphemes(text)
    return set_text(s, node, before .. text .. after)
end

local function add_row(s, node, text)
    if #node.data.rows >= MAX_ROWS then
        return array()
    end
    local rows = rows_of(node)
    rows[#rows + 1] = { text = text, color = next_color(rows) }
    node.data.rows = rows
    s.row, s.caret, s.fresh = #rows, #graphemes(text), text ~= ""
    return { update(refit(node)), { kind = "input.focus" } }
end

local function set_view(node, view)
    node.data.view, node.data.auto = view, false
    return { update(node) }
end

local function zoom_by(node, k, cx, cy)
    return set_view(node, Plot.zoom(node.data.view, k, cx, cy))
end

local function pan_by(node, fx, fy)
    local v = node.data.view
    local dx, dy = (v[2] - v[1]) * fx, (v[4] - v[3]) * fy
    return set_view(node, { v[1] + dx, v[2] + dx, v[3] + dy, v[4] + dy })
end

local function to_data(node, view, x, y)
    local L = Plot.layout(node)
    return view[1] + (x - L.l) / (L.r - L.l) * (view[2] - view[1]),
    view[3] + (y - L.b) / (L.t - L.b) * (view[4] - view[3])
end

local function selected_row(s, node)
    if s.row > 0 then
        return s.row
    end
    return #node.data.rows
end

local function cycle_color(s, node, row)
    local c = node.data.rows[row].color
    local next = 1
    for i, p in ipairs(Plot.PALETTE) do
        if color_key(p) == color_key(c) then
            next = i % #Plot.PALETTE + 1
        end
    end
    node.data.rows[row].color = copy4(Plot.PALETTE[next])
    return { update(node) }
end

local function delete_row(s, node, row)
    local rows = array()
    for i, r in ipairs(node.data.rows) do
        if i ~= row then
            rows[#rows + 1] = r
        end
    end
    if #rows == 0 then
        s.current, s.row = "", 0
        return { { kind = "node.delete", id = node.id } }
    end
    node.data.rows = rows
    s.row, s.caret = min(s.row, #rows), 0
    return { update(refit(node)) }
end

local function toggle_grid(_, node)
    node.data.grid = not node.data.grid
    return { update(node) }
end

local function fit_view(_, node)
    node.data.auto = true
    return { update(refit(node)) }
end

local NODE_COMMANDS = {
    add_fx = function(s, node)
        return add_row(s, node, "")
    end,
    add_param = function(s, node)
        return add_row(s, node, "(cos(t), sin(t))")
    end,
    fit = fit_view,
    zoom_in = function(_, node)
        return zoom_by(node, 0.8)
    end,
    zoom_out = function(_, node)
        return zoom_by(node, 1.25)
    end,
    grid = toggle_grid,
}

local ROW_COMMANDS = {
    color = cycle_color,
    delete_row = delete_row,
}

local function command(s, ctx, id)
    local node = find_node(ctx.nodes, s.current)
    if not node then
        return array()
    end

    local node_cmd = NODE_COMMANDS[id]
    if node_cmd then
        return node_cmd(s, node)
    end

    local row_cmd = ROW_COMMANDS[id]
    local row = selected_row(s, node)
    if row_cmd and row > 0 then
        return row_cmd(s, node, row)
    end

    return array()
end

local function try_drag_handle(s, cur, e)
    if not cur then
        return nil
    end
    local corner = hit_corner(cur, e.x, e.y)
    if not (corner or on_bar(cur, e.x, e.y)) then
        return nil
    end
    s.drag, s.corner = corner and "resize" or "move", corner or 0
    s.orig = { cur.x, cur.y, cur.width, cur.height }
    return { { kind = "input.focus" } }
end

local function create_graph_at(s, ctx, e)
    local node = new_graph(ctx, e.x, e.y, { "y = sin(x)" })
    s.current, s.row, s.caret, s.fresh = node.id, 1, #graphemes(node.data.rows[1].text), true
    return { { kind = "node.create", node = node }, { kind = "input.focus" } }
end

local function select_graph(s, ctx, cur, hit, e)
    local out = array()
    if cur and cur.id ~= hit.id then
        out = cleanup(cur)
    end
    s.current, s.fresh = hit.id, false
    local row = row_at(hit, e.y)
    if row then
        s.row, s.caret = row, caret_at(hit, row, e.x)
    else
        s.row, s.drag, s.view = 0, e.ctrl and "zoom" or "pan", copy4(hit.data.view)
        s.cx, s.cy = to_data(hit, hit.data.view, e.x, e.y)
    end
    out[#out + 1] = { kind = "input.focus" }
    return out
end

local function pointer_down(s, ctx, e)
    local cur = find_node(ctx.nodes, s.current)
    s.drag, s.sx, s.sy = "", e.x, e.y
    local handled = try_drag_handle(s, cur, e)
    if handled then
        return handled
    end
    local hit = find_hit(ctx.nodes, e.x, e.y)
    if not hit then
        if ctx.focused and cur then
            return finish(s, ctx)
        end
        return create_graph_at(s, ctx, e)
    end
    return select_graph(s, ctx, cur, hit, e)
end

local function drag_move(s, node, dx, dy)
    local o = s.orig
    node.x, node.y = o[1] + dx, o[2] + dy
    return { update(node) }
end

local function drag_resize(s, node, dx, dy)
    local o = s.orig
    local right = s.corner == 2 or s.corner == 3
    local bottom = s.corner == 3 or s.corner == 4
    local w = max(MIN_W, o[3] + (right and dx or -dx))
    local h = max(Plot.min_height(#node.data.rows), o[4] + (bottom and -dy or dy))
    node.x = right and o[1] or o[1] + o[3] - w
    node.y = bottom and o[2] or o[2] - o[4] + h
    node.width, node.height = w, h
    return { update(node) }
end

local function drag_pan(s, node, dx, dy)
    local L, v = Plot.layout(node), s.view
    local ux, uy = (v[2] - v[1]) / (L.r - L.l), (v[4] - v[3]) / (L.t - L.b)
    return set_view(node, { v[1] - dx * ux, v[2] - dx * ux, v[3] - dy * uy, v[4] - dy * uy })
end

local function drag_zoom(s, node, _, dy)
    return set_view(node, Plot.zoom(s.view, exp(-dy * 0.01), s.cx, s.cy))
end

local DRAG_HANDLERS = { move = drag_move, resize = drag_resize, pan = drag_pan, zoom = drag_zoom }

local function pointer_move(s, ctx, e)
    local node = find_node(ctx.nodes, s.current)
    if not node or s.drag == "" then
        return array()
    end
    local handler = DRAG_HANDLERS[s.drag]
    if not handler then
        return array()
    end
    return handler(s, node, e.x - s.sx, e.y - s.sy)
end

local function key_enter(s, node)
    if s.row == #node.data.rows then
        return add_row(s, node, "")
    end
    s.row, s.fresh = s.row + 1, false
    s.caret = #graphemes(node.data.rows[s.row].text)
    return array()
end

local function key_vertical(s, node, key)
    s.row = min(#node.data.rows, max(1, s.row + (key == "ArrowUp" and -1 or 1)))
    s.caret, s.fresh = min(s.caret, #graphemes(node.data.rows[s.row].text)), false
    return array()
end

local function key_delete(s, node, key)
    local text = node.data.rows[s.row].text
    if s.fresh then
        s.fresh, s.caret = false, 0
        return set_text(s, node, "")
    end
    local gs = graphemes(text)
    local at = key == "Backspace" and s.caret or s.caret + 1
    if at < 1 or at > #gs then
        return array()
    end
    table.remove(gs, at)
    s.caret = at - 1
    return set_text(s, node, table.concat(gs))
end

local function key_caret(s, node, key)
    local n = #graphemes(node.data.rows[s.row].text)
    local moves = { ArrowLeft = s.caret - 1, ArrowRight = s.caret + 1, Home = 0, End = n }
    if moves[key] then
        s.caret, s.fresh = min(n, max(0, moves[key])), false
    end
    return array()
end

local function key_editing(s, ctx, node, e)
    local key = e.key
    if e.ctrl and key == "v" then
        return { { kind = "clipboard.read" } }
    end
    if e.ctrl then
        return array()
    end
    if key == "Escape" then
        return finish(s, ctx)
    end
    if key == "Enter" then
        return key_enter(s, node)
    end
    if key == "ArrowUp" or key == "ArrowDown" then
        return key_vertical(s, node, key)
    end
    if key == "Backspace" or key == "Delete" then
        return key_delete(s, node, key)
    end
    return key_caret(s, node, key)
end

local function key_view(s, ctx, node, e)
    local key = e.key
    if e.ctrl then
        return array()
    end
    if key == "Escape" then
        return finish(s, ctx)
    end
    if key == "+" or key == "=" then
        return zoom_by(node, 0.8)
    end
    if key == "-" then
        return zoom_by(node, 1.25)
    end
    if key == "0" or key == "f" then
        return command(s, ctx, "fit")
    end
    if key == "Enter" then
        s.row, s.caret = 1, #graphemes(node.data.rows[1].text)
        return array()
    end
    if key == "Delete" or key == "Backspace" then
        s.current = ""
        return { { kind = "node.delete", id = node.id }, { kind = "input.blur" } }
    end
    local pans = { ArrowLeft = { -0.1, 0 }, ArrowRight = { 0.1, 0 }, ArrowUp = { 0, 0.1 }, ArrowDown = { 0, -0.1 } }
    if pans[key] then
        return pan_by(node, pans[key][1], pans[key][2])
    end
    return array()
end

local INK_MAX_STROKES = 300
local INK_MAX_POINTS = 1500
local INK_TOL = 0.6

local function r3(v)
    return floor(v * 1000 + 0.5) / 1000
end

local function add_ink(ctx, e)
    local node = find_node(ctx.nodes, e.node)
    if not node or type(e.points) ~= "table" or #e.points < 4 then
        return {}
    end
    local ink = node.data.ink or {}
    if #ink >= INK_MAX_STROKES then
        return {}
    end

    local L, view = Plot.layout(node), node.data.view
    local run = Plot.simplify(e.points, INK_TOL)
    if #run > INK_MAX_POINTS * 2 then
        return {}
    end

    local p = {}
    for i = 1, #run - 1, 2 do
        local x, y = Plot.to_math(view, L, run[i], run[i + 1])
        if not (Plot.finite(x) and Plot.finite(y)) then
            return {}
        end
        p[#p + 1] = r3(x)
        p[#p + 1] = r3(y)
    end
    if #p < 4 then
        return {}
    end

    ink[#ink + 1] = { w = e.width or 4, c = copy4(e.color or { 0.1, 0.1, 0.1, 1 }), p = p }
    node.data.ink = ink
    return { update(node) }
end

local function erase_ink(ctx, e)
    local node = find_node(ctx.nodes, e.node)
    if not node then
        return {}
    end
    local ink = node.data.ink or {}
    local k = Plot.ink_hit(ink, node.data.view, Plot.layout(node), e.x, e.y, e.tol or 6)
    if k then
        table.remove(ink, k)
        node.data.ink = ink
        return { update(node) }
    end
    if e.click then
        return { { kind = "node.delete", id = node.id } }
    end
    return {}
end

function P.activate(s)
    s.current, s.row, s.caret, s.fresh, s.drag = "", 0, 0, false, ""
    return {
        tools = { { id = "graph", label = "Graph", text_input = true, wheel = true } },
        object_types = { "graph" },
        ink = { "graph" },
        services = { "plot.v1" },
        commands = {
            { id = "add_fx", label = "+ y(x)" },
            { id = "add_param", label = "+ (x(t), y(t))" },
            { id = "fit", label = "Fit" },
            { id = "zoom_in", label = "Zoom +" },
            { id = "zoom_out", label = "Zoom -" },
            { id = "color", label = "Color" },
            { id = "grid", label = "Grid" },
            { id = "delete_row", label = "Delete row" },
        },
    }
end

local function handle_pointer(s, ctx, e)
    if e.phase == "down" then
        return pointer_down(s, ctx, e)
    end
    if e.phase == "move" then
        return pointer_move(s, ctx, e)
    end
    s.drag = ""
    return array()
end

local function handle_document_changed(s, node)
    if not node then
        s.current, s.row = "", 0
    else
        s.row = min(s.row, #node.data.rows)
    end
    return array()
end

local function handle_wheel(s, ctx, e)
    local hit = find_hit(ctx.nodes, e.x, e.y)
    if not hit then
        return array()
    end
    local cx, cy = to_data(hit, hit.data.view, e.x, e.y)
    return zoom_by(hit, exp(-e.delta * 0.004), cx, cy)
end

local function handle_row_input(s, ctx, node, e)
    if e.name == "text" then
        return insert_text(s, node, e.text)
    end
    if e.name == "clipboard" and e.ok then
        return insert_text(s, node, e.text)
    end
    if e.name == "key" then
        return key_editing(s, ctx, node, e)
    end
    return array()
end

local GLOBAL_EVENTS = {
    command = function(s, ctx, e)
        return command(s, ctx, e.id)
    end,
    blur = finish,
    ["tool.selected"] = finish,
    pointer = handle_pointer,
    ["document.changed"] = function(s, ctx)
        return handle_document_changed(s, find_node(ctx.nodes, s.current))
    end,
    wheel = handle_wheel,
}

function P.handle(s, e, ctx)
    if e.name == "ink.stroke" then
        return add_ink(ctx, e)
    end
    if e.name == "ink.erase" then
        return erase_ink(ctx, e)
    end
    local global = GLOBAL_EVENTS[e.name]
    if global then
        return global(s, ctx, e)
    end

    local node = find_node(ctx.nodes, s.current)
    if not node or not ctx.focused then
        return array()
    end
    if s.row > 0 and node.data.rows[s.row] then
        return handle_row_input(s, ctx, node, e)
    end
    if e.name == "key" then
        return key_view(s, ctx, node, e)
    end
    return array()
end

function P.render(s, node, ctx)
    local zoom = max(0.05, ctx.zoom or 1.0)
    local L = Plot.layout(node)
    local out = array()
    local view = node.data.view
    Plot.axes(out, view, L, node.data.grid ~= false)
    local fns = compile_rows(node.data.rows)
    local cap = floor(3000 / max(1, #valid(fns)))
    for i, fn in ipairs(fns) do
        if fn and not fn.error then
            Plot.curve(out, fn, view, L, zoom, node.data.rows[i].color, cap)
        end
    end
    Plot.ink(out, node.data.ink, view, L)
    for i, row in ipairs(node.data.rows) do
        local top = L.rows_top - (i - 1) * Plot.ROW
        local bad = fns[i] and fns[i].error
        Plot.rect(out, node.x + Plot.PAD + 2, top - 4, 10, 10, bad and ERROR_C or row.color)
        if bad then
            Plot.text(out, node.x + Plot.PAD + 5, top - 2.5, "!", 9, { 1, 1, 1, 1 })
        end
        if row.text ~= "" then
            Plot.text(out, L.text_x, top - 1, row.text, Plot.FONT, TEXT_C)
        end
    end
    return out
end

function P.overlay(s, ctx)
    local node = find_node(ctx.nodes, s.current)
    if not node then
        return array()
    end
    local zoom = max(0.05, ctx.zoom or 1.0)
    local out = array()
    local c = { 0.38, 0.45, 0.95, 0.65 }
    local lw = 1.4 / zoom
    local x0, x1, y0, y1 = node.x, node.x + node.width, node.y - node.height, node.y
    Plot.rect(out, x0, y1 + 8 / zoom, node.width, 6 / zoom, { 0.38, 0.45, 0.95, 0.28 })
    out[#out + 1] = { kind = "line", x1 = x0, y1 = y1, x2 = x1, y2 = y1, width = lw, color = c }
    out[#out + 1] = { kind = "line", x1 = x0, y1 = y0, x2 = x1, y2 = y0, width = lw, color = c }
    out[#out + 1] = { kind = "line", x1 = x0, y1 = y0, x2 = x0, y2 = y1, width = lw, color = c }
    out[#out + 1] = { kind = "line", x1 = x1, y1 = y0, x2 = x1, y2 = y1, width = lw, color = c }
    local hs = 8 / zoom
    for _, k in ipairs(corners(node)) do
        Plot.rect(out, k.x - hs / 2, k.y + hs / 2, hs, hs, { 0.25, 0.38, 0.96, 1.0 })
    end
    local row = node.data.rows[s.row]
    if not ctx.focused or not row then
        return out
    end

    local L = Plot.layout(node)
    local top = L.rows_top - (s.row - 1) * Plot.ROW
    Plot.rect(out, node.x + 2, top + 1, node.width - 4, Plot.ROW, { 0.22, 0.50, 1.0, s.fresh and 0.22 or 0.10 })
    if row.text == "" then
        Plot.text(out, L.text_x, top - 1, HINT, Plot.FONT, { 0.55, 0.57, 0.62, 0.8 })
    end
    local x = L.text_x + caret_x(row.text, s.caret)
    Plot.rect(out, x, top, 2 / zoom, Plot.FONT * 1.25, { 0.18, 0.48, 1.0, 1.0 })
    local fn = Expr.row(row.text)
    if fn and fn.error then
        local msg = fn.error
        Plot.rect(out, L.l + 2, L.t - 2, min(L.r - L.l - 4, #msg * 6.2 + 8), 16, { 1.0, 0.93, 0.93, 0.92 })
        Plot.text(out, L.l + 6, L.t - 3, msg, 10, ERROR_C)
    end
    return out
end

function P.service(s, name, args, ctx)
    if name ~= "plot.v1" then
        error("Unknown service: " .. tostring(name))
    end
    local texts = {}
    for i, t in ipairs(args.rows or {}) do
        if i <= MAX_ROWS and type(t) == "string" then
            texts[#texts + 1] = t
        end
    end
    if #texts == 0 then
        texts[1] = "y = x"
    end
    local node = new_graph(ctx, args.x or 0, args.y or 0, texts)
    node.width = max(MIN_W, args.width or node.width)
    node.height = max(Plot.min_height(#texts), args.height or node.height)
    return { value = { id = node.id }, commands = { { kind = "node.create", node = refit(node) } } }
end

function P.deactivate(s)
end

return P