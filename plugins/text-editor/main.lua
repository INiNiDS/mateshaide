local P = {}
local min, max, abs = math.min, math.max, math.abs

local function clamp(i, length)
	return max(0, min(i, length))
end

local function find_node(nodes, id)
	for _, node in ipairs(nodes) do
		if node.id == id then
			return node
		end
	end
end

local function geometry(node)
	local layout = text_layout(node.data.text, node.data.size)
	node.width = max(24.0, layout.width + 12.0)
	node.height = layout.height
	return node
end

local function new_node(s, ctx, x, y, text)
	text = replace_text(replace_text(text, "\r\n", "\n"), "\r", "\n")
	local node = geometry({
		id = ctx.new_id,
		type = "block",
		schema_version = 1,
		x = x,
		y = y,
		width = 24.0,
		height = s.size * 1.25,
		data = { text = text, size = s.size, color = { 0.92, 0.94, 0.98, 1.0 } },
	})
	s.current = node.id
	s.caret = #graphemes(text)
	s.anchor = s.caret
	return node
end

local function replace_selection(s, node, text)
	local a = graphemes(node.data.text)
	local lo = clamp(min(s.caret, s.anchor), #a)
	local hi = clamp(max(s.caret, s.anchor), #a)
	text = replace_text(replace_text(text, "\r\n", "\n"), "\r", "\n")
	local inserted = graphemes(text)
	local result = array()
	for i = 1, lo do
		result[#result + 1] = a[i]
	end
	for _, g in ipairs(inserted) do
		result[#result + 1] = g
	end
	for i = hi + 1, #a do
		result[#result + 1] = a[i]
	end
	node.data.text = join_text(result)
	s.caret = lo + #inserted
	s.anchor = s.caret
	s.preedit = ""
	return geometry(node)
end

local function selected_text(s, node)
	local a = graphemes(node.data.text)
	local lo = clamp(min(s.caret, s.anchor), #a)
	local hi = clamp(max(s.caret, s.anchor), #a)
	local result = array()
	for i = lo + 1, hi do
		result[#result + 1] = a[i]
	end
	return join_text(result)
end

local function index_at(node, x, y)
	local l = text_layout(node.data.text, node.data.size)
	local row = clamp(math.floor((node.y - y) / l.line_height), #l.lines - 1)
	local best, distance = 0, math.huge
	for i, c in ipairs(l.carets) do
		local d = abs(c.x - (x - node.x))
		if c.line == row and d < distance then
			distance = d
			best = i - 1
		end
	end
	return best
end

-- Mm, HIGH COMPLEXITY! HERE TOO... *crying emoji* TODO
local function move_cursor(s, node, key, ctrl, shift)
	local a = graphemes(node.data.text)
	s.caret = clamp(s.caret, #a)
	s.anchor = clamp(s.anchor, #a)
	if key == "ArrowLeft" then
		if not shift and s.anchor ~= s.caret then
			s.caret = min(s.anchor, s.caret)
		else
			s.caret = max(0, s.caret - 1)
		end
    elseif key == "ArrowRight" then
		if not shift and s.anchor ~= s.caret then
			s.caret = max(s.anchor, s.caret)
		else
			s.caret = min(#a, s.caret + 1)
		end
    else
		local l = text_layout(node.data.text, node.data.size)
		local c = l.carets[s.caret + 1]

		if key == "Home" then
			s.caret = ctrl and 0 or l.lines[c.line + 1].start
		elseif key == "End" then
			s.caret = ctrl and #a or l.lines[c.line + 1]["end"]
		else
			local target = c.line + (key == "ArrowUp" and -1 or 1)
			local best, distance = s.caret, math.huge

            for i, p in ipairs(l.carets) do
				local d = abs(p.x - c.x)
				if p.line == target and d < distance then
					best = i - 1
					distance = d
				end
            end

            s.caret = best
		end
	end

    if not shift then
		s.anchor = s.caret
	end
end

function P.activate(s)
	s.current = ""
	s.caret = 0
	s.anchor = 0
	s.dragging = false
	s.preedit = ""
	s.size = 26.0

	return {
		tools = { { id = "text", label = "Text", text_input = true } },
		object_types = { "block" },
		services = { "insert.v1" },
		commands = array(),
	}
end

-- Mm, HIGH COMPLEXITY! HERE TOO... *crying emoji* TODO
function P.handle(s, event, ctx)
	if event.name == "blur" or event.name == "tool.selected" then
		s.preedit = ""
		s.dragging = false
		return { { kind = "input.blur" } }
	end

	if event.name == "document.changed" then
		local node = find_node(ctx.nodes, s.current)
		if not node then
			s.current = ""
			s.caret = 0
			s.anchor = 0
		else
			local n = #graphemes(node.data.text)
			s.caret = clamp(s.caret, n)
			s.anchor = clamp(s.anchor, n)
		end
		s.preedit = ""
		return array()
	end

	if event.name == "pointer" then
		if event.phase == "up" then
			s.dragging = false
			return array()
		end

		if event.phase == "move" then
			local node = find_node(ctx.nodes, s.current)
			if s.dragging and node then
				s.caret = index_at(node, event.x, event.y)
			end
			return array()
		end

		local hit

		for i = #ctx.nodes, 1, -1 do
			local n = ctx.nodes[i]
			if
				n.type == "block"
				and event.x >= n.x
				and event.x <= n.x + n.width
				and event.y <= n.y
				and event.y >= n.y - n.height
			then
				hit = n
				break
			end
		end

		s.dragging = true

		if not hit then
			local node = new_node(s, ctx, event.x, event.y, "")
			return { { kind = "node.create", node = node }, { kind = "input.focus" } }
		end

		local same = s.current == hit.id
		s.current = hit.id
		s.caret = index_at(hit, event.x, event.y)
		if not event.shift or not same then
			s.anchor = s.caret
		end

		return { { kind = "input.focus" } }
	end

	if event.name == "ime.preedit" then
		s.preedit = event.text
		return array()
	end

	local node = find_node(ctx.nodes, s.current)

	if not node or not ctx.focused then
		return array()
	end

	if event.name == "text" or (event.name == "clipboard" and event.ok) then
		if event.text == "" then
			return array()
		end
		return { { kind = "node.update", node = replace_selection(s, node, event.text) } }
	end

	if event.name ~= "key" then
		return array()
	end

	local key = event.key
	if event.ctrl then
		if key == "a" then
			s.anchor = 0
			s.caret = #graphemes(node.data.text)
			return array()
		end

		if key == "v" then
			return { { kind = "clipboard.read" } }
		end

		if key == "c" or key == "x" then
			if s.caret == s.anchor then
				return array()
			end

			local result = { { kind = "clipboard.write", text = selected_text(s, node) } }
			if key == "x" then
				result[#result + 1] = { kind = "node.update", node = replace_selection(s, node, "") }
			end

			return result
		end
	end

	if key == "Escape" then
		s.preedit = ""
		s.dragging = false
		return { { kind = "input.blur" } }
	end

	if
		key == "ArrowLeft"
		or key == "ArrowRight"
		or key == "ArrowUp"
		or key == "ArrowDown"
		or key == "Home"
		or key == "End"
	then
		move_cursor(s, node, key, event.ctrl, event.shift)
		return array()
	end

	if key == "Backspace" or key == "Delete" then
		local length = #graphemes(node.data.text)
		s.caret = clamp(s.caret, length)
		s.anchor = clamp(s.anchor, length)
		if s.caret == s.anchor then
			s.anchor = key == "Backspace" and max(0, s.caret - 1) or min(length, s.caret + 1)
		end
		if s.caret == s.anchor then
			return array()
		end
		return { { kind = "node.update", node = replace_selection(s, node, "") } }
	end

	if key == "Enter" or key == "Tab" then
		return { { kind = "node.update", node = replace_selection(s, node, key == "Enter" and "\n" or "\t") } }
	end

	return array()
end

function P.render(s, node, ctx)
	return { { kind = "text", x = node.x, y = node.y, text = node.data.text, size = node.data.size, color = node.data.color } }
end
function P.overlay(s, ctx)
	if not ctx.focused then
		return array()
	end

    local node = find_node(ctx.nodes, s.current)

    if not node then
		return array()
	end

    local l = text_layout(node.data.text, node.data.size)
	local caret = clamp(s.caret, #l.carets - 1)
	local lo = clamp(min(s.anchor, s.caret), #l.carets - 1)
	local hi = clamp(max(s.anchor, s.caret), #l.carets - 1)
	local out = array()
	if lo ~= hi then
		for _, row in ipairs(l.lines) do
			local a, b = max(lo, row.start), min(hi, row["end"])
			if a <= b and lo <= row["end"] and hi >= row.start then
				local x1 = l.carets[a + 1].x
				local x2 = b == row["end"] and row.width or l.carets[b + 1].x
				out[#out + 1] = {
					kind = "rect",
					x = node.x + min(x1, x2),
					y = node.y - row.y,
					width = max(abs(x2 - x1), 3.0),
					height = l.line_height,
					color = { 0.22, 0.50, 1.0, 0.30 },
				}
			end
		end
	end

	local c = l.carets[caret + 1]
	out[#out + 1] =
		{ kind = "rect", x = node.x + c.x, y = node.y - c.y, width = 1.5 / ctx.zoom, height = l.line_height, color = {
			0.6,
			0.8,
			1.0,
			1.0,
		} }

	if s.preedit ~= "" then
		out[#out + 1] =
			{ kind = "text", x = node.x + c.x, y = node.y - c.y, text = s.preedit, size = node.data.size, color = {
				0.6,
				0.8,
				1.0,
				1.0,
			} }

		local m = text_layout(s.preedit, node.data.size)
		out[#out + 1] = {
			kind = "line",
			x1 = node.x + c.x,
			y1 = node.y - c.y - l.line_height,
			x2 = node.x + c.x + m.width,
			y2 = node.y - c.y - l.line_height,
			width = 1.0 / ctx.zoom,
			color = { 0.6, 0.8, 1.0, 1.0 },
        }
	end

	return out
end

function P.service(s, name, args, ctx)
	assert(name == "insert.v1", "Unknown service")
	local node = new_node(s, ctx, args.x, args.y, args.text)
	return { value = { id = node.id }, commands = { { kind = "node.create", node = node } } }
end

function P.deactivate(s) end

return P
