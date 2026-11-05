local P = {}

local TEMPLATES = {
	{ id = "tpl_int", label = "∫ Integral", action = "insert_int", snippet = "∫_{a}^{b} f(x)dx" },
	{ id = "tpl_sum", label = "∑ Sum", action = "insert_sum", snippet = "∑_{i=1}^{n} a_{i}" },
	{ id = "tpl_prod", label = "∏ Prod", action = "insert_prod", snippet = "∏_{k=1}^{n} x_{k}" },
	{ id = "tpl_frac", label = "a//b Frac", action = "insert_frac", snippet = "{x+1}//{x-1}" },
	{ id = "tpl_sqrt", label = "√ Root", action = "insert_sqrt", snippet = "√{x^{2}+y^{2}}" },
	{ id = "tpl_lim", label = "lim", action = "insert_lim", snippet = "lim_{x→∞} {1}//{x}" },
	{ id = "tpl_sup", label = "xⁿ Sup", action = "sup_mode", snippet = "x^{2}" },
	{ id = "tpl_sub", label = "xₙ Sub", action = "sub_mode", snippet = "x_{0}" },
	{ id = "tpl_size_up", label = "Text A+", action = "size_up", snippet = nil },
	{ id = "tpl_size_down", label = "Text A-", action = "size_down", snippet = nil },
	{ id = "tpl_del", label = "Delete Block", action = "delete_block", snippet = nil },
}

function P.activate(s)
	s.active_snippet = "∫_{0}^{∞} {sin(x)}//{x} dx = {π}//{2}"

	local commands = array()
	for _, item in ipairs(TEMPLATES) do
		commands[#commands + 1] = {
			id = item.id,
			label = item.label,
		}
	end

	return {
		tools = { { id = "math_builder", label = "Math Stamp", text_input = false } },
		services = array(),
		object_types = array(),
		commands = commands,
	}
end

function P.handle(s, event, ctx)
	if event.name == "command" then
		for _, item in ipairs(TEMPLATES) do
			if event.id == item.id then
				if item.snippet then
					s.active_snippet = item.snippet
				end
				return {
					{
						kind = "call",
						plugin = "org.matesha.text",
						service = "command.v1",
						args = {
							action = item.action,
						},
					},
				}
			end
		end
	end

	if event.name == "pointer" and event.phase == "down" then
		return {
			{
				kind = "call",
				plugin = "org.matesha.text",
				service = "insert.v1",
				args = {
					x = event.x,
					y = event.y,
					text = s.active_snippet,
					color = { 0.12, 0.15, 0.22, 1.0 },
				},
			},
		}
	end

	return array()
end

function P.deactivate(s) end

return P
