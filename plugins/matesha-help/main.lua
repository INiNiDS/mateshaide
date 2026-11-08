local P = {}

local min, max = math.min, math.max

local SELF = "org.matesha.help"
local PANEL_W = 780
local TOC_W = 220
local PAD = 14
local HEADER_H = 46
local FOOTER_H = 46
local TOC_ROW = 26
local TOC_GAP = 3
local BODY_SIZE = 14
local LINE = 1.5
local HEADING = { 22, 17, 16 }
local MAX_BATCH_BYTES = 30000
local CODE_INDENT = 12
local BULLET_INDENT = 16
local layout_cache = {}
local HEADING_LEAD = 0.6
local HEADING_LINE = 1.35
local GAP_H = 0.6

function P.activate(s)
    s.open = false
    s.page = ""
    s.scroll = 0
    return {
        commands = { { id = "toggle", label = "? Help" } },
        services = { "open.v1" },
    }
end

local function open_page(s, page)
    s.open = true
    if type(page) == "string" and page ~= "" then
        s.page = page
    end
    s.scroll = 0
end

function P.handle(s, event, ctx)
    local name = event.name
    if name == "command" and event.id == "toggle" then
        s.open = not s.open
        s.scroll = 0
    elseif name == "panel.click" then
        local id = event.id
        if id == "close" then
            s.open = false
        elseif string.sub(id, 1, 5) == "page:" then
            open_page(s, string.sub(id, 6))
        end
    elseif name == "panel.scroll" then
        s.scroll = max(0, min(s.scroll + (event.dy or 0), event.max or 0))
    end
    return array()
end

function P.service(s, name, args, ctx)
    if name == "open.v1" then
        open_page(s, args and args.page)
        return { value = { ok = true }, commands = array() }
    end
    error("Unknown service: " .. tostring(name))
end

local function page_title(page)
    local title = string.match(page.text, "^#%s+([^\n]+)")
    return title or (string.gsub(page.file, "%.md$", ""))
end

local function collect_pages(raw)
    local pages = {}
    for _, p in ipairs(raw or {}) do
        pages[#pages + 1] = {
            id = p.plugin .. "/" .. p.file,
            plugin = p.plugin,
            group = p.plugin == SELF and "MateshaIDE" or (p.plugin_name .. (p.active and "" or " (off)")),
            file = p.file,
            text = p.text,
            title = page_title(p),
        }
    end
    table.sort(pages, function(a, b)
        local sa, sb = a.plugin == SELF, b.plugin == SELF
        if sa ~= sb then
            return sa
        end
        if a.group ~= b.group then
            return a.group < b.group
        end
        return a.file < b.file
    end)
    return pages
end

local function measure(paragraphs, size)
    local result, i = {}, 1
    while i <= #paragraphs do
        local batch, bytes, first = {}, 0, i
        while i <= #paragraphs and (#batch == 0 or bytes + #paragraphs[i] + 1 <= MAX_BATCH_BYTES) do
            batch[#batch + 1] = paragraphs[i]
            bytes = bytes + #paragraphs[i] + 1
            i = i + 1
        end
        local joined = table.concat(batch, "\n")
        local gs = graphemes(joined)
        local carets = text_layout(joined, size).carets
        local k = 1
        for n = first, i - 1 do
            local pg, px = {}, {}
            while k <= #gs and gs[k] ~= "\n" do
                pg[#pg + 1] = gs[k]
                px[#px + 1] = carets[k].x
                k = k + 1
            end
            px[#px + 1] = (carets[k] or carets[#carets]).x
            result[n] = { gs = pg, xs = px }
            k = k + 1
        end
    end
    return result
end

local function wrap(m, width)
    local gs, xs = m.gs, m.xs
    if #gs == 0 then
        return { "" }
    end
    local lines, start, x0, space = {}, 1, xs[1], nil
    local i = 1
    while i <= #gs do
        if gs[i] == " " then
            space = i
        end
        if xs[i + 1] - x0 > width and i > start then
            local brk = (space and space >= start) and space or (i - 1)
            lines[#lines + 1] = table.concat(gs, "", start, brk)
            start = brk + 1
            x0 = xs[start]
            space = nil
            i = start
        else
            i = i + 1
        end
    end
    if start <= #gs then
        lines[#lines + 1] = table.concat(gs, "", start, #gs)
    end
    return lines
end

local function parse_page(text)
    local blocks = {}
    for raw in string.gmatch(text .. "\n", "([^\n]*)\n") do
        local hashes, heading = string.match(raw, "^(#+)%s+(.*)$")
        if hashes then
            local size = HEADING[min(#hashes, 3)]
            blocks[#blocks + 1] = { kind = "heading", size = size, text = heading, color = #hashes == 1 and "accent" or "text" }
        elseif raw == "" then
            blocks[#blocks + 1] = { kind = "gap" }
        elseif string.sub(raw, 1, 4) == "    " then
            blocks[#blocks + 1] = { kind = "code", size = BODY_SIZE - 1, text = string.sub(raw, 5) }
        else
            local bullet = string.match(raw, "^[-*]%s+(.*)$")
            local body = string.gsub(bullet or raw, "%*%*", "")
            blocks[#blocks + 1] = { kind = "text", size = BODY_SIZE, text = body, bullet = bullet ~= nil }
        end
    end
    return blocks
end

local function measure_blocks(blocks)
    local by_size = {}
    for _, b in ipairs(blocks) do
        if b.kind == "heading" or b.kind == "text" then
            local group = by_size[b.size] or { text = {}, blocks = {} }
            by_size[b.size] = group
            group.text[#group.text + 1] = b.text
            group.blocks[#group.blocks + 1] = b
        end
    end
    for size, group in pairs(by_size) do
        for n, m in ipairs(measure(group.text, size)) do
            group.blocks[n].measure = m
        end
    end
end

local function add_toc(items, pages, current, height)
    local y, group = HEADER_H, nil
    for _, page in ipairs(pages) do
        if page.group ~= group then
            group = page.group
            if y + 18 > height - FOOTER_H then
                return
            end
            items[#items + 1] = { kind = "text", x = PAD, y = y + 2, size = 11, text = group, color = "muted" }
            y = y + 18
        end
        if y + TOC_ROW > height - FOOTER_H then
            return
        end
        items[#items + 1] = {
            kind = "button",
            id = "page:" .. page.id,
            x = PAD,
            y = y,
            width = TOC_W - PAD,
            height = TOC_ROW,
            label = page.title,
            selected = page == current,
        }
        y = y + TOC_ROW + TOC_GAP
    end
end

local function emit_heading(out, y, b, width)
    if #out > 0 then
        y = y + b.size * HEADING_LEAD
    end
    for _, line in ipairs(wrap(b.measure, width)) do
        out[#out + 1] = { x = 0, y = y, size = b.size, text = line, color = b.color }
        y = y + b.size * HEADING_LINE
    end
    return y
end

local function emit_gap(_, y)
    return y + BODY_SIZE * GAP_H
end

local function emit_code(out, y, b)
    out[#out + 1] = { x = CODE_INDENT, y = y, size = b.size, text = b.text, color = "muted" }
    return y + b.size * LINE
end

local function emit_text(out, y, b, width)
    local indent = b.bullet and BULLET_INDENT or 0
    for k, line in ipairs(wrap(b.measure, width - indent)) do
        if b.bullet and k == 1 then
            out[#out + 1] = { x = 2, y = y, size = BODY_SIZE, text = "•", color = "muted" }
        end
        out[#out + 1] = { x = indent, y = y, size = BODY_SIZE, text = line, color = "text" }
        y = y + BODY_SIZE * LINE
    end
    return y
end

local EMITTERS = {
    heading = emit_heading,
    gap = emit_gap,
    code = emit_code,
    text = emit_text,
}

local function layout_page(text, width)
    local blocks = parse_page(text)
    measure_blocks(blocks)
    local out, y = {}, 0
    for _, b in ipairs(blocks) do
        y = EMITTERS[b.kind](out, y, b, width)
    end
    return out, y
end

local function cached_layout(page, width)
    local e = layout_cache[page.id]
    if e and e.width == width and e.text == page.text then
        return e.lines, e.total
    end
    local lines, total = layout_page(page.text, width)
    layout_cache[page.id] = { text = page.text, width = width, lines = lines, total = total }
    return lines, total
end

local function lower_bound(lines, y_min)
    local lo, hi = 1, #lines + 1
    while lo < hi do
        local mid = math.floor((lo + hi) / 2)
        if lines[mid].y < y_min then
            lo = mid + 1
        else
            hi = mid
        end
    end
    return lo
end

local function add_body(items, page, s, width, height)
    local lines, total = cached_layout(page, width)
    local view = height - HEADER_H - FOOTER_H
    local scroll_max = max(0, total - view + PAD)
    local scroll = min(s.scroll, scroll_max)
    local x0 = TOC_W + PAD * 2
    for i = lower_bound(lines, scroll - 2), #lines do
        local l = lines[i]
        if l.y > scroll + view then
            break
        end
        local y = HEADER_H + l.y - scroll
        if y + l.size * 1.3 <= height - FOOTER_H then
            items[#items + 1] = { kind = "text", x = x0 + l.x, y = y, size = l.size, text = l.text, color = l.color }
        end
    end
    return scroll_max
end

function P.deactivate(s)
    layout_cache = {}
end

local function add_body(items, page, s, width, height)
    local lines, total = layout_page(page.text, width)
    local view = height - HEADER_H - FOOTER_H
    local scroll_max = max(0, total - view + PAD)
    local scroll = min(s.scroll, scroll_max)
    local x0 = TOC_W + PAD * 2
    for _, l in ipairs(lines) do
        local y = HEADER_H + l.y - scroll
        if y >= HEADER_H - 2 and y + l.size * 1.3 <= height - FOOTER_H then
            items[#items + 1] = { kind = "text", x = x0 + l.x, y = y, size = l.size, text = l.text, color = l.color }
        end
    end
    return scroll_max
end

local function add_footer(items, pages, index, width, height)
    local y = height - FOOTER_H + 8
    local prev, next = pages[index - 1], pages[index + 1]
    if prev then
        items[#items + 1] = {
            kind = "button",
            id = "page:" .. prev.id,
            x = TOC_W + PAD * 2,
            y = y,
            width = 150,
            height = 30,
            label = "< " .. prev.title,
        }
    end
    if next then
        items[#items + 1] = {
            kind = "button",
            id = "page:" .. next.id,
            x = width - PAD - 150,
            y = y,
            width = 150,
            height = 30,
            label = next.title .. " >",
        }
    end
    items[#items + 1] = {
        kind = "text",
        x = PAD,
        y = y + 8,
        size = 11,
        text = index .. " / " .. #pages .. "   F1 — close",
        color = "muted",
    }
end

function P.panel(s, ctx)
    if not s.open then
        return nil
    end
    local width = min(PANEL_W, ctx.width - 32)
    local height = ctx.height - ctx.top - 64
    if width < 360 or height < 220 then
        return nil
    end
    local pages = collect_pages(ctx.pages)
    local items = array()
    items[#items + 1] = { kind = "text", x = PAD, y = 14, size = 16, text = "Help", color = "accent" }
    items[#items + 1] = { kind = "button", id = "close", x = width - PAD - 30, y = 8, width = 30, height = 30, label = "x" }
    items[#items + 1] = { kind = "rect", x = PAD, y = HEADER_H - 6, width = width - 2 * PAD, height = 1, color = "border" }

    if #pages == 0 then
        items[#items + 1] = {
            kind = "text",
            x = PAD,
            y = HEADER_H + 4,
            size = BODY_SIZE,
            text = "No help pages found.",
            color = "muted",
        }
        return { x = ctx.width - 16 - width, y = ctx.top, width = width, height = height, items = items }
    end

    local index = 1
    for i, page in ipairs(pages) do
        if page.id == s.page then
            index = i
        end
    end
    local page = pages[index]
    items[#items + 1] = {
        kind = "text",
        x = TOC_W + PAD * 2,
        y = 14,
        size = 13,
        text = page.group .. " / " .. page.title,
        color = "muted",
    }

    add_toc(items, pages, page, height)
    local scroll_max = add_body(items, page, s, width - TOC_W - PAD * 3, height)
    add_footer(items, pages, index, width, height)

    return {
        x = ctx.width - 16 - width,
        y = ctx.top,
        width = width,
        height = height,
        scroll_max = scroll_max,
        items = items,
    }
end

function P.deactivate(s)
end

return P
