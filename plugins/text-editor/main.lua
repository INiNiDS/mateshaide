local P = {}

local min, max, abs, floor = math.min, math.max, math.abs, math.floor

local HANDLE_SCREEN_RADIUS = 8.0
local MOVE_BAR_SCREEN_HEIGHT = 12.0
local MIN_FONT_SIZE = 8.0
local MAX_FONT_SIZE = 220.0

local COLOR_PALETTE = {
	{
		id = "color_black",
		label = "Ink",
		color = { 0.12, 0.15, 0.22, 1.0 },
	},
	{
		id = "color_blue",
		label = "Blue",
		color = { 0.18, 0.40, 0.88, 1.0 },
	},
	{
		id = "color_red",
		label = "Red",
		color = { 0.85, 0.18, 0.20, 1.0 },
	},
	{
		id = "color_green",
		label = "Green",
		color = { 0.10, 0.55, 0.30, 1.0 },
	},
}

local PALETTE_BY_ID = {}
for _, item in ipairs(COLOR_PALETTE) do
	PALETTE_BY_ID[item.id] = item.color
end

local CURSOR_KEYS = {
	ArrowLeft = true,
	ArrowRight = true,
	ArrowUp = true,
	ArrowDown = true,
	Home = true,
	End = true,
}

local RESERVED_CTRL_KEYS = {
	a = true,
	c = true,
	v = true,
	x = true,
	z = true,
	y = true,
	s = true,
	o = true,
	p = true,
	e = true,
}

local AUTO_WORDS = {
	int = "∫",
	iint = "∬",
	oint = "∮",
	sum = "∑",
	prod = "∏",
	sqrt = "√",
	inf = "∞",
	infty = "∞",
	pi = "π",
	alpha = "α",
	beta = "β",
	gamma = "γ",
	delta = "δ",
	Delta = "Δ",
	epsilon = "ε",
	theta = "θ",
	lambda = "λ",
	mu = "μ",
	sigma = "σ",
	omega = "ω",
	Omega = "Ω",
	pm = "±",
	cdot = "·",
	times = "×",
	in_ = "∈",
	forall = "∀",
	exists = "∃",
	nabla = "∇",
	partial = "∂",
}

local SYMBOL_PAIRS = {
	["<="] = "≤",
	[">="] = "≥",
	["!="] = "≠",
	["->"] = "→",
	["=>"] = "⇒",
	["+-"] = "±",
}

local BIG_OPS = {
	["∫"] = true,
	["∬"] = true,
	["∮"] = true,
	["∑"] = true,
	["∏"] = true,
}


local BASELINE_RATIO = (1.25 - (1900 + 500) / 2048) / 2 + 1900 / 2048
local LIMIT_ASCENT = 1536 / 2048
local LIMIT_DESCENT = 437 / 2048

local OP_INK = {
	["∑"] = { 1456 / 2048, -270 / 2048 },
	["∏"] = { 1456 / 2048, -270 / 2048 },
	["∫"] = { 1558 / 2048, -437 / 2048 },
	["∬"] = { 1558 / 2048, -437 / 2048 },
	["∮"] = { 1558 / 2048, -437 / 2048 },
}

local BIG_OP_SPACING1 = 0.111
local BIG_OP_SPACING2 = 0.167
local BIG_OP_SPACING3 = 0.200
local BIG_OP_SPACING4 = 0.600
local BIG_OP_SPACING5 = 0.100
local OP_RAISE = 0.16

local SIDE_LIMIT_OPS = { ["∫"] = true, ["∬"] = true, ["∮"] = true }
local INT_SUP_KERN = 0.08
local INT_SUB_KERN = 0.02

local SUB_SHIFT = 0.15
local SUP_SHIFT = 0.42

local function baseline_of(dy, size)
	return dy - BASELINE_RATIO * size
end

local function dy_for_baseline(baseline, size)
	return baseline + BASELINE_RATIO * size
end

local SIZE_FACTORS = {
	size_up = 1.18,
	size_down = 1.0 / 1.18,
}

local function copy_color(color)
	assert(type(color) == "table" and #color == 4, "Expected RGBA color")
	local result = {}
	for i = 1, 4 do
		local value = color[i]
		assert(type(value) == "number" and value >= 0 and value <= 1, "RGBA values must be between 0 and 1")
		result[i] = value
	end
	return result
end

local function clamp(i, length)
	return max(0, min(i, length))
end

local function clamp_range(v, lo, hi)
	return max(lo, min(v, hi))
end

local function find_node(nodes, id)
	for _, node in ipairs(nodes) do
		if node.id == id then
			return node
		end
	end
end

local function reset_drag(s)
	s.dragging = false
	s.interaction = ""
end

local function is_word_char(g)
	if not g or #g ~= 1 then
		return false
	end
	local b = string.byte(g, 1)
	return (b >= 48 and b <= 57) or (b >= 65 and b <= 90) or (b >= 97 and b <= 122) or b == 95
end

local function is_expr_Atom_char(g)
	if not g or g == "" or g == " " or g == "\t" or g == "\n" then
		return false
	end
	if g == "+" or g == "=" or g == "," or g == ";" then
		return false
	end
	return true
end

local function measure_text_width(text, size)
	if text == "" then
		return 0.0
	end
	local l = text_layout(text, clamp_range(size, 4.0, 512.0))
	return l.width
end

local function find_matching_brace(gs, open_idx, total)
	local depth = 0
	for j = open_idx, total do
		if gs[j] == "{" then
			depth = depth + 1
		elseif gs[j] == "}" then
			depth = depth - 1
			if depth == 0 then
				return j
			end
		end
	end
	return nil
end

local function parse_script_group(gs, idx, total)
	if idx > total then
		return idx, idx - 1, idx, idx - 1
	end

	if gs[idx] == "{" then
		local close = find_matching_brace(gs, idx, total)
		if close then
			return idx, close, idx + 1, close - 1
		end
		return idx, total, idx + 1, total
	end

	local j = idx
	while j <= total and is_word_char(gs[j]) do
		j = j + 1
	end
	if j == idx then
		return idx, idx, idx, idx
	end
	return idx, j - 1, idx, j - 1
end

local function strip_braces(gs, s_idx, e_idx)
	local out = ""
	for k = s_idx, e_idx do
		if gs[k] ~= "{" and gs[k] ~= "}" then
			out = out .. gs[k]
		end
	end
	return out
end

local function new_line_builder(gs, line_start, line_end, base_size)
	local b = {
		gs = gs,
		base = base_size,
		carets = {},
		primitives = array(),
		x = 0.0,
		ascent = base_size * 1.0,
		descent = base_size * 0.25,
	}
	for i = line_start, line_end + 1 do
		b.carets[i] = { x = 0.0, y = 0.0, h = base_size * 1.25 }
	end
	return b
end

local function set_caret(b, i, x, y, h)
	b.carets[i] = { x = x, y = y, h = h }
end

local function push_primitive(b, p)
	b.primitives[#b.primitives + 1] = p
end

local function visible_chars(gs, s_idx, e_idx, hide_braces)
	local chars, map, hidden = array(), {}, {}
	for i = s_idx, e_idx do
		local g = gs[i]
		if hide_braces and (g == "{" or g == "}") then
			hidden[#hidden + 1] = i
		else
			chars[#chars + 1] = g
			map[#chars] = i
		end
	end
	return chars, map, hidden
end

local function caret_height(font_size)
	return font_size * 1.25
end

local function place_carets_in_range(b, first, last, x, y, h)
	for i = first, last do
		if b.carets[i] then
			set_caret(b, i, x, y, h)
		end
	end
end

local function place_char_carets(b, chars, map, lay, x, y, h)
	for k = 1, #chars do
		local c = lay.carets[k]
		set_caret(b, map[k], x + (c and c.x or 0.0), y, h)
	end
end

local function place_brace_carets(b, s_idx, e_idx, x, y, h)
	local gs = b.gs
	for i = s_idx, e_idx do
		if gs[i] == "}" then
			set_caret(b, i, x, y, h)
		end
	end
end

local function emit_span(b, s_idx, e_idx, x, y_offset, font_size, hide_braces)
	font_size = clamp_range(font_size, 4.0, 512.0)
	local h = caret_height(font_size)

	if e_idx < s_idx then
		place_carets_in_range(b, s_idx, s_idx, x, y_offset, h)
		return 0.0
	end

	local chars, map, hidden = visible_chars(b.gs, s_idx, e_idx, hide_braces)
	for _, i in ipairs(hidden) do
		set_caret(b, i, x, y_offset, h)
	end

	local str = join_text(chars)
	if str == "" then
		place_carets_in_range(b, s_idx, e_idx + 1, x, y_offset, h)
		return 0.0
	end

	local lay = text_layout(str, font_size)
	push_primitive(b, {
		kind = "text",
		dx = x,
		dy = y_offset,
		text = str,
		size = font_size,
	})

	place_char_carets(b, chars, map, lay, x, y_offset, h)

	local end_c = lay.carets[#chars + 1]
	local end_x = x + (end_c and end_c.x or lay.width)

	if hide_braces then
		place_brace_carets(b, s_idx, e_idx, end_x, y_offset, h)
	end

	if b.carets[e_idx + 1] then
		set_caret(b, e_idx + 1, end_x, y_offset, h)
	end

	return lay.width
end


local function find_slash_pos(gs, i, line_end)
	if not is_expr_Atom_char(gs[i]) or gs[i] == "/" then
		return nil
	end

	local paren = 0
	for scan = i, line_end do
		local g = gs[scan]
		if g == "(" or g == "{" then
			paren = paren + 1
		elseif g == ")" or g == "}" then
			paren = max(0, paren - 1)
		end

		if paren == 0 and scan + 1 <= line_end and g == "/" and gs[scan + 1] == "/" then
			return scan > i and scan or nil
		end
		if paren == 0 and (g == " " or g == "+" or g == "=" or g == "-") and scan > i then
			return nil
		end
	end
	return nil
end

local function find_denominator_end(gs, den_s, line_end)
	local den_e = den_s - 1
	local paren = 0
	while den_e + 1 <= line_end do
		local ng = gs[den_e + 1]
		if ng == "(" or ng == "{" then
			paren = paren + 1
		elseif ng == ")" or ng == "}" then
			if paren == 0 then
				break
			end
			paren = paren - 1
		elseif paren == 0 and (ng == " " or ng == "+" or ng == "-" or ng == "=" or ng == ",") then
			break
		end
		den_e = den_e + 1
	end
	return den_e
end

local function layout_fraction(b, i, slash_pos, line_end)
	local gs, base = b.gs, b.base
	local num_s, num_e = i, slash_pos - 1
	local den_s = slash_pos + 2
	local den_e = find_denominator_end(gs, den_s, line_end)

	local frac_size = clamp_range(base * 0.78, 4.0, 512.0)
	local num_str = strip_braces(gs, num_s, num_e)
	local den_str = strip_braces(gs, den_s, den_e)

	local num_w = max(10.0, measure_text_width(num_str, frac_size))
	local den_w = max(10.0, measure_text_width(den_str, frac_size))
	local bar_w = max(num_w, den_w) + base * 0.45
	local pad_x = base * 0.12

	local num_y = base * 0.55
	local den_y = -base * 0.52
	local bar_y = -base * 0.48

	local num_x = b.x + pad_x + (bar_w - num_w) * 0.5
	local den_x = b.x + pad_x + (bar_w - den_w) * 0.5

	emit_span(b, num_s, num_e, num_x, num_y, frac_size, true)
	set_caret(b, slash_pos, num_x + num_w, num_y, frac_size * 1.25)
	set_caret(b, slash_pos + 1, den_x, den_y, frac_size * 1.25)
	emit_span(b, den_s, den_e, den_x, den_y, frac_size, true)

	push_primitive(b, {
		kind = "line",
		x1 = b.x + pad_x,
		y1 = bar_y,
		x2 = b.x + pad_x + bar_w,
		y2 = bar_y,
		width = max(1.5, base * 0.07),
	})

	b.ascent = max(b.ascent, base * 1.45)
	b.descent = max(b.descent, base * 0.85)

	b.x = b.x + bar_w + pad_x * 2.0
	set_caret(b, den_e + 1, b.x, 0.0, base * 1.25)
	return den_e + 1
end

local function read_limits(gs, idx, line_end)
	local sub, sup
	for _ = 1, 2 do
		if idx > line_end or (gs[idx] ~= "_" and gs[idx] ~= "^") then
			break
		end
		local kind = gs[idx]
		local raw_s, raw_e, inner_s, inner_e = parse_script_group(gs, idx + 1, line_end)
		local info = { op_i = idx, raw_s = raw_s, raw_e = raw_e, s = inner_s, e = inner_e }
		if kind == "_" then
			sub = info
		else
			sup = info
		end
		idx = raw_e + 1
	end
	return sub, sup, idx
end

local function limit_width(gs, info, lim_size)
	if not info then
		return 0.0
	end
	local s_txt = ""
	for k = info.s, info.e do
		s_txt = s_txt .. gs[k]
	end
	return max(8.0, measure_text_width(s_txt, lim_size))
end


local function grow_line(b, ink_top, ink_bottom)
	local pad = b.base * BIG_OP_SPACING5
	b.ascent = max(b.ascent, ink_top + b.base + pad)
	b.descent = max(b.descent, -ink_bottom - b.base + pad)
end

local function layout_big_op(b, i, line_end)
	local gs, base = b.gs, b.base
	local op_char = gs[i]
	local op_size = clamp_range(base * 1.38, 4.0, 512.0)
	local op_w = measure_text_width(op_char, op_size)
	local ink = OP_INK[op_char] or OP_INK["∫"]

	local sub_info, sup_info, next_idx = read_limits(gs, i + 1, line_end)

	local lim_size = clamp_range(base * 0.62, 4.0, 512.0)
	local sup_w = limit_width(gs, sup_info, lim_size)
	local sub_w = limit_width(gs, sub_info, lim_size)

	local op_dy = base * OP_RAISE
	local op_base = baseline_of(op_dy, op_size)
	local op_top = op_base + ink[1] * op_size
	local op_bottom = op_base + ink[2] * op_size
	local lim_asc = LIMIT_ASCENT * lim_size
	local lim_desc = LIMIT_DESCENT * lim_size

	local side = SIDE_LIMIT_OPS[op_char]
	local block_w, op_x
	if side then
		block_w = op_w + max(sup_w, sub_w) + base * 0.22
		op_x = b.x + base * 0.05
	else
		block_w = max(op_w, max(sup_w, sub_w)) + base * 0.22
		op_x = b.x + (block_w - op_w) * 0.5
	end

	push_primitive(b, {
		kind = "text",
		dx = op_x,
		dy = op_dy,
		text = op_char,
		size = op_size,
	})
	set_caret(b, i, op_x, 0.0, base * 1.25)
	grow_line(b, op_top, op_bottom)

	if sup_info then
		local sx, sup_base
		if side then
			sx = op_x + op_w + op_size * INT_SUP_KERN
			sup_base = op_top - lim_asc * 0.6
		else
			sx = b.x + (block_w - sup_w) * 0.5
			local gap = max(BIG_OP_SPACING1 * base, BIG_OP_SPACING3 * base - lim_desc)
			sup_base = op_top + gap + lim_desc
		end
		local sy = dy_for_baseline(sup_base, lim_size)
		set_caret(b, sup_info.op_i, sx, sy, lim_size * 1.25)
		emit_span(b, sup_info.raw_s, sup_info.raw_e, sx, sy, lim_size, true)
		grow_line(b, sup_base + lim_asc, sup_base - lim_desc)
	end

	if sub_info then
		local sx, sub_base
		if side then
			sx = op_x + op_w - op_size * INT_SUB_KERN
			sub_base = op_bottom + lim_desc * 0.5
		else
			sx = b.x + (block_w - sub_w) * 0.5
			local gap = max(BIG_OP_SPACING2 * base, BIG_OP_SPACING4 * base - lim_asc)
			sub_base = op_bottom - gap - lim_asc
		end
		local sy = dy_for_baseline(sub_base, lim_size)
		set_caret(b, sub_info.op_i, sx, sy, lim_size * 1.25)
		emit_span(b, sub_info.raw_s, sub_info.raw_e, sx, sy, lim_size, true)
		grow_line(b, sub_base + lim_asc, sub_base - lim_desc)
	end

	b.x = b.x + block_w
	set_caret(b, next_idx, b.x, 0.0, base * 1.25)
	return next_idx
end

local function layout_sqrt(b, i, line_end)
	local base = b.base
	local rad_w = emit_span(b, i, i, b.x, 0.0, base * 1.1, false)
	b.x = b.x + rad_w

	local raw_s, raw_e = parse_script_group(b.gs, i + 1, line_end)
	if raw_e < raw_s then
		return i + 1
	end

	local body_w = emit_span(b, raw_s, raw_e, b.x, 0.0, base, true)
	push_primitive(b, {
		kind = "line",
		x1 = b.x - 2.0,
		y1 = base * 0.04,
		x2 = b.x + body_w + 2.0,
		y2 = base * 0.04,
		width = max(1.5, base * 0.065),
	})
	b.x = b.x + body_w + base * 0.1
	return raw_e + 1
end

local function layout_script(b, i, line_end)
	local gs, base = b.gs, b.base
	local is_sup = gs[i] == "^"
	local raw_s, raw_e = parse_script_group(gs, i + 1, line_end)

	if is_sup then
		b.ascent = max(b.ascent, base * 1.25)
	else
		b.descent = max(b.descent, base * 0.55)
	end

	set_caret(b, i, b.x, sc_y, sc_size * 1.25)
	local w = emit_span(b, raw_s, raw_e, b.x, sc_y, sc_size, true)

	if is_sup then
		b.ascent = max(b.ascent, base * 1.25)
	else
		b.descent = max(b.descent, base * 0.55)
	end

	b.x = b.x + max(w, 4.0)
	set_caret(b, raw_e + 1, b.x, 0.0, base * 1.25)
	return raw_e + 1
end

local function text_run_end(gs, i, line_end)
	local run_e = i
	while run_e + 1 <= line_end do
		local ng = gs[run_e + 1]
		if BIG_OPS[ng] or ng == "^" or ng == "_" or ng == "√" then
			break
		end
		if ng == " " then
			run_e = run_e + 1
			break
		end
		if run_e + 2 <= line_end and gs[run_e + 1] == "/" and gs[run_e + 2] == "/" then
			break
		end
		run_e = run_e + 1
	end
	return run_e
end

local function layout_text_run(b, i, line_end)
	local run_e = text_run_end(b.gs, i, line_end)
	b.x = b.x + emit_span(b, i, run_e, b.x, 0.0, b.base, false)
	return run_e + 1
end

local function build_visual_line(gs, line_start, line_end, base_size)
	local b = new_line_builder(gs, line_start, line_end, base_size)

	local i = line_start
	while i <= line_end do
		set_caret(b, i, b.x, 0.0, base_size * 1.25)

		local slash_pos = find_slash_pos(gs, i, line_end)
		if slash_pos then
			i = layout_fraction(b, i, slash_pos, line_end)
		elseif BIG_OPS[gs[i]] then
			i = layout_big_op(b, i, line_end)
		elseif gs[i] == "√" then
			i = layout_sqrt(b, i, line_end)
		elseif gs[i] == "^" or gs[i] == "_" then
			i = layout_script(b, i, line_end)
		else
			i = layout_text_run(b, i, line_end)
		end
	end

	set_caret(b, line_end + 1, b.x, 0.0, base_size * 1.25)

	return {
		width = b.x,
		ascent = b.ascent,
		descent = b.descent,
		height = b.ascent + b.descent,
		carets = b.carets,
		primitives = b.primitives,
	}
end

local function build_node_layout(text, size)
	size = clamp_range(size or 26.0, MIN_FONT_SIZE, MAX_FONT_SIZE)
	local gs = graphemes(text)
	local n = #gs

	local line_ranges = array()
	local start_i = 1
	for i = 1, n do
		if gs[i] == "\n" then
			line_ranges[#line_ranges + 1] = { s = start_i, e = i - 1 }
			start_i = i + 1
		end
	end
	line_ranges[#line_ranges + 1] = { s = start_i, e = n }

	local carets = array()
	for i = 1, n + 1 do
		carets[i] = { x = 0.0, y = 0.0, h = size * 1.25, line = 0 }
	end

	local lines = array()
	local primitives = array()
	local total_width = 0.0
	local y_cursor = 0.0

	for row_idx, r in ipairs(line_ranges) do
		local vline = build_visual_line(gs, r.s, r.e, size)
		local extra_top = max(0.0, vline.ascent - size)
		local baseline_top = y_cursor + extra_top
		local row_h = vline.height

		for i = r.s, r.e + 1 do
			local c = vline.carets[i] or { x = 0.0, y = 0.0, h = size * 1.25 }
			carets[i] = {
				x = c.x,
				y = baseline_top - c.y,
				h = c.h,
				line = row_idx - 1,
			}
		end

		for _, p in ipairs(vline.primitives) do
			if p.kind == "text" then
				primitives[#primitives + 1] = {
					kind = "text",
					dx = p.dx,
					dy = baseline_top - p.dy,
					text = p.text,
					size = p.size,
				}
			elseif p.kind == "line" then
				primitives[#primitives + 1] = {
					kind = "line",
					x1 = p.x1,
					y1 = baseline_top - p.y1,
					x2 = p.x2,
					y2 = baseline_top - p.y2,
					width = p.width,
				}
			end
		end

		lines[#lines + 1] = {
			start = r.s - 1,
			["end"] = r.e,
			y = baseline_top,
			top = y_cursor,
			height = row_h,
			width = vline.width,
		}

		total_width = max(total_width, vline.width)
		y_cursor = y_cursor + row_h + size * 0.12
	end

	return {
		width = total_width,
		height = max(size * 1.3, y_cursor),
		line_height = size * 1.25,
		carets = carets,
		lines = lines,
		primitives = primitives,
	}
end

local function geometry(node)
	local layout = build_node_layout(node.data.text, node.data.size)
	node.width = max(36.0, layout.width + 16.0)
	node.height = max(node.data.size * 1.3, layout.height + 8.0)
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
		width = 36.0,
		height = s.size * 1.3,
		data = {
			text = text,
			size = s.size,
			color = copy_color(s.color),
		},
	})

	s.current = node.id
	s.caret = #graphemes(text)
	s.anchor = s.caret

	return node
end


local function replace_pair_symbol(node, s, gs, caret)
	local two = gs[caret - 1] .. gs[caret]
	local sym = SYMBOL_PAIRS[two]
	if not sym then
		return false
	end

	local out = array()
	for i = 1, caret - 2 do
		out[#out + 1] = gs[i]
	end
	out[#out + 1] = sym
	for i = caret + 1, #gs do
		out[#out + 1] = gs[i]
	end

	node.data.text = join_text(out)
	s.caret = caret - 1
	s.anchor = s.caret
	return true
end

local function replace_word(node, s, gs, caret)
	local w_start = caret
	while w_start > 1 and is_word_char(gs[w_start - 1]) do
		w_start = w_start - 1
	end

	local parts = array()
	for i = w_start, caret do
		parts[#parts + 1] = gs[i]
	end
	local rep = AUTO_WORDS[join_text(parts)]
	if not rep then
		return
	end

	local out = array()
	for i = 1, w_start - 1 do
		out[#out + 1] = gs[i]
	end
	local rep_gs = graphemes(rep)
	for _, rg in ipairs(rep_gs) do
		out[#out + 1] = rg
	end
	for i = caret + 1, #gs do
		out[#out + 1] = gs[i]
	end

	node.data.text = join_text(out)
	s.caret = (w_start - 1) + #rep_gs
	s.anchor = s.caret
end

local function apply_math_autoreplace(node, s)
	local gs = graphemes(node.data.text)
	local caret = clamp(s.caret, #gs)
	if caret < 2 then
		return
	end

	if replace_pair_symbol(node, s, gs, caret) then
		return
	end
	replace_word(node, s, gs, caret)
end

local function replace_selection(s, node, text, skip_autoreplace)
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

	if not skip_autoreplace then
		apply_math_autoreplace(node, s)
	end

	return geometry(node)
end

local function insert_into_script(s, node, script_op, content)
	local gs = graphemes(node.data.text)
	local caret = clamp(s.caret, #gs)

	local open_brace = nil
	local depth = 0
	for i = caret, 1, -1 do
		if gs[i] == "}" then
			depth = depth + 1
		elseif gs[i] == "{" then
			if depth == 0 then
				open_brace = i
				break
			end
			depth = depth - 1
		end
	end

	if open_brace and open_brace > 1 and gs[open_brace - 1] == script_op then
		return replace_selection(s, node, content, false)
	end

	local c_gs = graphemes(content)
	local out = array()
	for i = 1, caret do
		out[#out + 1] = gs[i]
	end
	out[#out + 1] = script_op
	out[#out + 1] = "{"
	for _, cg in ipairs(c_gs) do
		out[#out + 1] = cg
	end
	out[#out + 1] = "}"
	for i = caret + 1, #gs do
		out[#out + 1] = gs[i]
	end

	node.data.text = join_text(out)
	s.caret = caret + 2 + #c_gs
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
	local l = build_node_layout(node.data.text, node.data.size)
	local local_x = x - node.x - 6.0
	local local_y = node.y - y - 4.0

	local best, best_score = 0, math.huge
	for i, c in ipairs(l.carets) do
		local dx = abs(c.x - local_x)
		local cy = c.y + (c.h or l.line_height) * 0.5
		local dy = abs(cy - local_y) * 2.2
		local score = dx + dy
		if score < best_score then
			best_score = score
			best = i - 1
		end
	end
	return best
end

local function horizontal_caret(s, length, key, shift)
	local has_selection = s.anchor ~= s.caret
	if key == "ArrowLeft" then
		if not shift and has_selection then
			return min(s.anchor, s.caret)
		end
		return max(0, s.caret - 1)
	end
	if not shift and has_selection then
		return max(s.anchor, s.caret)
	end
	return min(length, s.caret + 1)
end

local function move_cursor(s, node, key, ctrl, shift)
	local gs = graphemes(node.data.text)
	local length = #gs

	s.caret = clamp(s.caret, length)
	s.anchor = clamp(s.anchor, length)

	if key == "ArrowLeft" or key == "ArrowRight" then
		s.caret = horizontal_caret(s, length, key, shift)
	else
		local l = build_node_layout(node.data.text, node.data.size)
		local c = l.carets[s.caret + 1] or { x = 0.0, y = 0.0, line = 0 }
		if key == "Home" then
			s.caret = ctrl and 0 or l.lines[c.line + 1].start
		elseif key == "End" then
			s.caret = ctrl and length or l.lines[c.line + 1]["end"]
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

local function build_commands()
	local commands = array()

	for _, item in ipairs(COLOR_PALETTE) do
		commands[#commands + 1] = {
			id = item.id,
			label = item.label,
			style = {
				background = copy_color(item.color),
				text = { 1.0, 1.0, 1.0, 1.0 },
			},
		}
	end

	commands[#commands + 1] = { id = "size_down", label = "A-" }
	commands[#commands + 1] = { id = "size_up", label = "A+" }

	return commands
end

function P.activate(s)
	s.current = ""
	s.caret = 0
	s.anchor = 0
	s.dragging = false
	s.interaction = "" -- "", "select", "resize", "move"
	s.resize_corner = 0
	s.start_x = 0.0
	s.start_y = 0.0
	s.orig_x = 0.0
	s.orig_y = 0.0
	s.orig_w = 0.0
	s.orig_h = 0.0
	s.orig_size = 26.0
	s.preedit = ""
	s.size = 28.0
	s.shift_sup = true
	s.pending_script = ""
	s.color = { 0.12, 0.15, 0.22, 1.0 }

	return {
		tools = { { id = "text", label = "Text / Math", text_input = true } },
		object_types = { "block" },
		services = { "insert.v1", "command.v1" },
		commands = build_commands(),
	}
end

local function node_corners(n)
	return {
		{ id = 1, x = n.x, y = n.y },
		{ id = 2, x = n.x + n.width, y = n.y },
		{ id = 3, x = n.x + n.width, y = n.y - n.height },
		{ id = 4, x = n.x, y = n.y - n.height },
	}
end

local function hit_resize_corner(node, x, y, radius)
	if not node then
		return nil
	end
	for _, c in ipairs(node_corners(node)) do
		if abs(x - c.x) <= radius and abs(y - c.y) <= radius then
			return c.id
		end
	end
	return nil
end

local function hit_move_border(node, x, y, bar_h)
	if not node then
		return false
	end
	return x >= node.x - 4.0 and x <= node.x + node.width + 4.0 and y <= node.y + bar_h and y >= node.y - 4.0
end

local function find_hit(nodes, x, y)
	for i = #nodes, 1, -1 do
		local n = nodes[i]
		if
			n.type == "block"
			and x >= n.x - 4.0
			and x <= n.x + n.width + 4.0
			and y <= n.y + 6.0
			and y >= n.y - n.height - 4.0
		then
			return n
		end
	end
end

local function resize_node(s, node, event)
	local dx = event.x - s.start_x
	local dy = s.start_y - event.y

	local right = s.resize_corner == 2 or s.resize_corner == 3
	local bottom = s.resize_corner == 3 or s.resize_corner == 4
	local dw = right and dx or -dx
	local dh = bottom and dy or -dy

	local scale_x = (s.orig_w + dw) / max(20.0, s.orig_w)
	local scale_y = (s.orig_h + dh) / max(20.0, s.orig_h)
	local scale = max(0.25, (scale_x + scale_y) * 0.5)

	node.data.size = clamp_range(s.orig_size * scale, MIN_FONT_SIZE, MAX_FONT_SIZE)
	s.size = node.data.size
	geometry(node)
	return { { kind = "node.update", node = node } }
end

local function move_node(s, node, event)
	node.x = s.orig_x + (event.x - s.start_x)
	node.y = s.orig_y + (event.y - s.start_y)
	return { { kind = "node.update", node = node } }
end

local function pointer_move(s, ctx, event)
	local node = find_node(ctx.nodes, s.current)
	if not s.dragging or not node then
		return array()
	end

	if s.interaction == "resize" then
		return resize_node(s, node, event)
	end
	if s.interaction == "move" then
		return move_node(s, node, event)
	end

	s.caret = index_at(node, event.x, event.y)
	return array()
end

local function begin_node_drag(s, node, event)
	if not node then
		return false
	end

	local corner = hit_resize_corner(node, event.x, event.y, HANDLE_SCREEN_RADIUS)
	if corner then
		s.dragging = true
		s.interaction = "resize"
		s.resize_corner = corner
		s.start_x = event.x
		s.start_y = event.y
		s.orig_w = node.width
		s.orig_h = node.height
		s.orig_size = node.data.size
		return true
	end

	if hit_move_border(node, event.x, event.y, MOVE_BAR_SCREEN_HEIGHT) then
		s.dragging = true
		s.interaction = "move"
		s.start_x = event.x
		s.start_y = event.y
		s.orig_x = node.x
		s.orig_y = node.y
		return true
	end

	return false
end

local function pointer_down(s, ctx, event)
	local current = find_node(ctx.nodes, s.current)
	if begin_node_drag(s, current, event) then
		return { { kind = "input.focus" } }
	end

	local hit = find_hit(ctx.nodes, event.x, event.y)
	s.dragging = true
	s.interaction = "select"

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

local function handle_pointer(s, ctx, event)
	if event.phase == "up" then
		reset_drag(s)
		return array()
	end
	if event.phase == "move" then
		return pointer_move(s, ctx, event)
	end
	return pointer_down(s, ctx, event)
end

local function handle_delete(s, node, key)
	local gs = graphemes(node.data.text)
	local length = #gs

	s.caret = clamp(s.caret, length)
	s.anchor = clamp(s.anchor, length)

	if s.caret == s.anchor then
		s.anchor = key == "Backspace" and max(0, s.caret - 1) or min(length, s.caret + 1)
	end

	if s.caret == s.anchor then
		return array()
	end

	return { { kind = "node.update", node = replace_selection(s, node, "", true) } }
end

local function apply_color(s, ctx, color)
	s.color = copy_color(color)
	local node = find_node(ctx.nodes, s.current)
	if not node then
		return array()
	end
	node.data.color = copy_color(color)
	return { { kind = "node.update", node = node } }
end

local function resolve_target(s, ctx)
	local node = find_node(ctx.nodes, s.current)
	if not node and #ctx.nodes > 0 then
		node = ctx.nodes[#ctx.nodes]
		s.current = node.id
		s.caret = #graphemes(node.data.text)
		s.anchor = s.caret
	end
	return node
end

local function apply_size(s, node, factor)
	s.size = clamp_range(s.size * factor, MIN_FONT_SIZE, MAX_FONT_SIZE)
	if not node then
		return array()
	end
	node.data.size = clamp_range(node.data.size * factor, MIN_FONT_SIZE, MAX_FONT_SIZE)
	geometry(node)
	return { { kind = "node.update", node = node }, { kind = "input.focus" } }
end

local function delete_target(s, node)
	if not node then
		return array()
	end
	local del_id = node.id
	s.current = ""
	return { { kind = "node.delete", id = del_id }, { kind = "input.blur" } }
end

local function apply_insert(s, ctx, node, action_id, custom_text)
	local is_script = action_id == "sup_mode" or action_id == "sub_mode"
	local is_custom = action_id == "insert_text" and custom_text ~= nil
	if not (is_script or is_custom) then
		return array()
	end

	local created = false
	if not node then
		node = new_node(s, ctx, -80.0, 40.0, "")
		created = true
	end

	if action_id == "sup_mode" then
		insert_into_script(s, node, "^", "")
	elseif action_id == "sub_mode" then
		insert_into_script(s, node, "_", "")
	else
		replace_selection(s, node, custom_text, false)
	end

	return {
		{ kind = created and "node.create" or "node.update", node = node },
		{ kind = "input.focus" },
	}
end

local function execute_editor_action(s, ctx, action_id, custom_text)
	local color = PALETTE_BY_ID[action_id]
	if color then
		return apply_color(s, ctx, color)
	end

	local node = resolve_target(s, ctx)

	local factor = SIZE_FACTORS[action_id]
	if factor then
		return apply_size(s, node, factor)
	end

	if action_id == "delete_block" then
		return delete_target(s, node)
	end

	return apply_insert(s, ctx, node, action_id, custom_text)
end

local function copy_or_cut(s, node, key)
	if s.caret == s.anchor then
		return array()
	end
	local result = { { kind = "clipboard.write", text = selected_text(s, node) } }
	if key == "x" then
		result[#result + 1] = { kind = "node.update", node = replace_selection(s, node, "", true) }
	end
	return result
end

local function handle_ctrl_key(s, node, key)
	if key == "a" then
		s.anchor = 0
		s.caret = #graphemes(node.data.text)
		return array()
	end
	if key == "v" then
		return { { kind = "clipboard.read" } }
	end
	if key == "c" or key == "x" then
		return copy_or_cut(s, node, key)
	end
	if #key == 1 and not RESERVED_CTRL_KEYS[key] and is_word_char(key) then
		return { { kind = "node.update", node = insert_into_script(s, node, "_", key) } }
	end
	return nil
end

local function track_shift_char(s, event)
	local key = event.key
	if event.shift and #key == 1 and is_word_char(key) then
		s.last_shift_char = true
	else
		s.last_shift_char = false
	end
end

local function handle_key(s, node, event)
	local key = event.key

	if event.ctrl then
		local result = handle_ctrl_key(s, node, key)
		if result then
			return result
		end
	end

	if key == "Escape" then
		s.preedit = ""
		reset_drag(s)
		return { { kind = "input.blur" } }
	end

	if CURSOR_KEYS[key] then
		move_cursor(s, node, key, event.ctrl, event.shift)
		return array()
	end

	if key == "Backspace" or key == "Delete" then
		return handle_delete(s, node, key)
	end

	if key == "Enter" or key == "Tab" then
		local text = key == "Enter" and "\n" or "    "
		return { { kind = "node.update", node = replace_selection(s, node, text, true) } }
	end

	track_shift_char(s, event)
	return array()
end

local function handle_document_changed(s, ctx)
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

local function try_superscript_input(s, node, text)
	local gs = graphemes(node.data.text)
	local prev = s.caret > 0 and gs[s.caret] or ""
	if prev == "" or prev == " " or prev == "\n" then
		return nil
	end
	return { { kind = "node.update", node = insert_into_script(s, node, "^", string.lower(text)) } }
end

local function handle_text_input(s, node, text, is_typed)
	if text == "" then
		return array()
	end

	if is_typed and s.last_shift_char and #graphemes(text) == 1 then
		s.last_shift_char = false
		local sup = try_superscript_input(s, node, text)
		if sup then
			return sup
		end
	end

	s.last_shift_char = false
	return { { kind = "node.update", node = replace_selection(s, node, text, false) } }
end

function P.handle(s, event, ctx)
	local name = event.name

	if name == "command" then
		return execute_editor_action(s, ctx, event.id, nil)
	end

	if name == "blur" or name == "tool.selected" then
		s.preedit = ""
		reset_drag(s)
		return { { kind = "input.blur" } }
	end

	if name == "document.changed" then
		return handle_document_changed(s, ctx)
	end

	if name == "pointer" then
		return handle_pointer(s, ctx, event)
	end

	if name == "ime.preedit" then
		s.preedit = event.text
		return array()
	end

	local node = find_node(ctx.nodes, s.current)
	if not node or not ctx.focused then
		return array()
	end

	if name == "text" then
		return handle_text_input(s, node, event.text, true)
	end
	if name == "clipboard" and event.ok then
		return handle_text_input(s, node, event.text, false)
	end

	if name == "key" then
		return handle_key(s, node, event)
	end

	return array()
end

function P.render(s, node, ctx)
	local l = build_node_layout(node.data.text, node.data.size)
	local out = array()
	local base_x = node.x + 6.0
	local base_y = node.y - 4.0

	for _, p in ipairs(l.primitives) do
		if p.kind == "text" then
			out[#out + 1] = {
				kind = "text",
				x = base_x + p.dx,
				y = base_y - p.dy,
				text = p.text,
				size = p.size,
				color = node.data.color,
			}
		elseif p.kind == "line" then
			out[#out + 1] = {
				kind = "line",
				x1 = base_x + p.x1,
				y1 = base_y - p.y1,
				x2 = base_x + p.x2,
				y2 = base_y - p.y2,
				width = p.width,
				color = node.data.color,
			}
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
	local border_c = { 0.38, 0.45, 0.95, 0.65 }
	local handle_c = { 0.25, 0.38, 0.96, 1.0 }
	local lw = 1.4 / zoom

	out[#out + 1] = {
		kind = "rect",
		x = node.x,
		y = node.y + 8.0 / zoom,
		width = node.width,
		height = 6.0 / zoom,
		color = { 0.38, 0.45, 0.95, 0.28 },
	}
	out[#out + 1] =
		{ kind = "line", x1 = node.x, y1 = node.y, x2 = node.x + node.width, y2 = node.y, width = lw, color = border_c }
	out[#out + 1] = {
		kind = "line",
		x1 = node.x,
		y1 = node.y - node.height,
		x2 = node.x + node.width,
		y2 = node.y - node.height,
		width = lw,
		color = border_c,
	}
	out[#out + 1] = {
		kind = "line",
		x1 = node.x,
		y1 = node.y,
		x2 = node.x,
		y2 = node.y - node.height,
		width = lw,
		color = border_c,
	}
	out[#out + 1] = {
		kind = "line",
		x1 = node.x + node.width,
		y1 = node.y,
		x2 = node.x + node.width,
		y2 = node.y - node.height,
		width = lw,
		color = border_c,
	}

	local hs = 8.0 / zoom
	for _, c in ipairs(node_corners(node)) do
		out[#out + 1] = {
			kind = "rect",
			x = c.x - hs * 0.5,
			y = c.y + hs * 0.5,
			width = hs,
			height = hs,
			color = handle_c,
		}
	end

	if not ctx.focused then
		return out
	end

	local l = build_node_layout(node.data.text, node.data.size)
	local caret = clamp(s.caret, #l.carets - 1)
	local lo = clamp(min(s.anchor, s.caret), #l.carets - 1)
	local hi = clamp(max(s.anchor, s.caret), #l.carets - 1)
	local base_x = node.x + 6.0
	local base_y = node.y - 4.0

	if lo ~= hi then
		for idx = lo + 1, hi do
			local c1 = l.carets[idx]
			local c2 = l.carets[idx + 1] or c1
			if c1 and c2 and c1.line == c2.line then
				local rx = base_x + min(c1.x, c2.x)
				local rw = max(abs(c2.x - c1.x), 4.0)
				out[#out + 1] = {
					kind = "rect",
					x = rx,
					y = base_y - c1.y,
					width = rw,
					height = c1.h or l.line_height,
					color = { 0.22, 0.50, 1.0, 0.28 },
				}
			end
		end
	end

	local c = l.carets[caret + 1] or { x = 0.0, y = 0.0, h = l.line_height }
	out[#out + 1] = {
		kind = "rect",
		x = base_x + c.x,
		y = base_y - c.y,
		width = 2.0 / zoom,
		height = c.h or l.line_height,
		color = { 0.18, 0.48, 1.0, 1.0 },
	}

	return out
end

function P.service(s, name, args, ctx)
	if name == "insert.v1" then
		local node = new_node(s, ctx, args.x or 0.0, args.y or 0.0, args.text or "")
		if args.color ~= nil then
			node.data.color = copy_color(args.color)
		end
		if args.size ~= nil then
			node.data.size = clamp_range(args.size, MIN_FONT_SIZE, MAX_FONT_SIZE)
			geometry(node)
		end
		return {
			value = { id = node.id },
			commands = {
				{ kind = "node.create", node = node },
				{ kind = "input.focus" },
			},
		}
	elseif name == "command.v1" then
		local cmds = execute_editor_action(s, ctx, args.action or "", args.text)
		return {
			value = { ok = true, current = s.current },
			commands = cmds,
		}
	end

	error("Unknown service: " .. tostring(name))
end

function P.deactivate(s) end

return P
