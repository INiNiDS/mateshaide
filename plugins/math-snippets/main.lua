local P = {}

function P.activate(s)
    s.index = 1
    s.formulas = { "x² + y² = r²", "a² + b² = c²", "f(x) = sin(x)", "E = mc²" }
    s.last = ""
    return {
        tools = { { id = "formula", label = "Formula", text_input = false } },
        services = array(),
        object_types = array(),
        commands = { { id = "next", label = "Next formula" } },
    }
end

function P.handle(s, event, ctx)
    if event.name == 'command' and event.id == 'next' then
        s.index = s.index % #s.formulas + 1
    end
    if event.name == 'pointer' and event.phase == 'down' then
        return { { kind = 'call', plugin = 'org.matesha.text', service = 'insert.v1', args = { x = event.x, y = event.y, text = s.formulas[s.index] }, token = 'inserted' } }
    end
    if event.name == 'service.result' and event.token == 'inserted' then
        s.last = event.value.id
    end
    return array()
end

function P.deactivate(s) end

return P