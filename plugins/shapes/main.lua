local P = {}

function P.activate(s)
	s.current = ""
	s.start_x = 0.0
	s.start_y = 0.0
	s.circle = false

    return {
		tools = { { id = "shape", label = "Shape", text_input = false } },
		object_types = { "geometry" },
		services = array(),
		commands = { { id = "toggle", label = "Rect / Circle" } },
	}
end

function P.handle(s, event, ctx)
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
					data = { circle = s.circle, color = { 0.24, 0.64, 0.96, 0.75 } },
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

    return { { kind = "rect", x = node.x, y = node.y, width = node.width, height = node.height, color = node.data.color } }
end

function P.deactivate(s) end

return P
