local P = {}

local COLOR_PALETTE = {
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
	{
		id = "color_blue",
		label = "Blue",
		color = { 0.18, 0.40, 0.88, 1.0 },
	},
	{
		id = "color_black",
		label = "Black",
		color = { 0.12, 0.15, 0.22, 1.0 },
	},
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

local function color_commands()
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

	return commands
end

local function color_from_event(event)
	if event.name ~= "command" then
		return nil
	end

	for _, item in ipairs(COLOR_PALETTE) do
		if event.id == item.id then
			return copy_color(item.color)
		end
	end

	return nil
end

function P.activate(s)
	s.current = ""
	s.start_x = 0.0
	s.start_y = 0.0
	s.circle = false

	s.color = { 0.24, 0.64, 0.96, 0.75 }

	local commands = color_commands()
	commands[#commands + 1] = {
		id = "toggle",
		label = "Rect / Circle",
	}

	return {
		tools = { { id = "shape", label = "Shape", text_input = false } },
		object_types = { "geometry" },
		services = array(),
		commands = commands,
	}
end

function P.handle(s, event, ctx)
	local color = color_from_event(event)

	if color then
		s.color = color
		return array()
	end

	if event.name == "command" and event.id == "toggle" then
		s.circle = not s.circle
	end

	if event.name == "blur" or event.name == "document.changed" then
		s.current = ""
	end

	if event.name ~= "pointer" then
		return array()
	end

	if event.phase == "down" then
		s.current = ctx.new_id
		s.start_x = event.x
		s.start_y = event.y
		return {
			{
				kind = "node.create",
				node = {
					id = ctx.new_id,
					type = "geometry",
					x = event.x,
					y = event.y,
					width = 2.0,
					height = 2.0,
					data = {
						circle = s.circle,
						color = copy_color(s.color),
					},
				},
			},
		}
	end

	if s.current == "" then
		return array()
	end

	local node

	for _, n in ipairs(ctx.nodes) do
		if n.id == s.current then
			node = n
		end
	end

	if not node then
		s.current = ""
		return array()
	end

	node.x = math.min(s.start_x, event.x)
	node.y = math.max(s.start_y, event.y)
	node.width = math.max(2.0, math.abs(event.x - s.start_x))
	node.height = math.max(2.0, math.abs(event.y - s.start_y))

	if event.phase == "up" then
		s.current = ""
	end

	return { { kind = "node.update", node = node } }
end
function P.render(s, node, ctx)
	if node.data.circle then
		return {
			{
				kind = "circle",
				x = node.x + node.width / 2,
				y = node.y - node.height / 2,
				radius = math.min(node.width, node.height) / 2,
				color = node.data.color,
			},
		}
	end

	return {
		{ kind = "rect", x = node.x, y = node.y, width = node.width, height = node.height, color = node.data.color },
	}
end

function P.deactivate(s) end

return P
