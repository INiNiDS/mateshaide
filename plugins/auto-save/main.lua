local P = {}

local DEBOUNCE_MS = 2000
local INTERVAL_MS = 30000
local RETRY_MS = 10000

function P.activate(s)
    s.enabled = true
    s.started = false
    s.saved = ""
    s.seen = ""
    s.changed_at = 0
    s.dirty_since = nil
    s.failed_at = nil
    s.now = 0
    s.status = ""
    s.refactoring = false
    return {
        commands = {
            { id = "save_now", label = "Save Now" },
            { id = "restore", label = "Restore" },
            { id = "toggle", label = "On/Off" },
        }
    }
end

local function set_status(s, out, text)
    if s.status ~= text then
        s.status = text
        out[#out + 1] = { kind = "status.set", text = text }
    end
end

local function save(s, out)
    out[#out + 1] = { kind = "document.save" }
    set_status(s, out, "saving...")
end

local function first_tick(s, event, out)
    s.started = true
    s.seen, s.saved = event.stamp, event.stamp
    if s.enabled and event.empty then
        s.restoring = true
        out[#out + 1] = { kind = "document.restore" }
    end
    set_status(s, out, s.enabled and "on" or "off")
end

local function on_tick(s, event, out)
    s.now = event.now
    if not s.started then
        return first_tick(s, event, out)
    end
    if not s.enabled then
        return set_status(s, out, "off")
    end
    if event.stamp ~= s.seen then
        s.seen = event.stamp
        s.changed_at = event.now
    end
    if event.stamp == s.saved then
        s.dirty_since = nil
        return
    end
    s.dirty_since = s.dirty_since or event.now
    local quiet = event.now - s.changed_at >= DEBOUNCE_MS
    local overdue = event.now - s.dirty_since >= INTERVAL_MS
    local waiting = s.failed_at and event.now - s.failed_at < RETRY_MS
    if (quiet or overdue) and not waiting then
        save(s, out)
    else
        set_status(s, out, s.failed_at and "error" or "unsaved")
    end
end

local function on_saved(s, event, out)
    if event.ok then
        s.saved = event.stamp
        s.failed_at = nil
        s.dirty_since = nil
        set_status(s, out, "saved")
    else
        s.failed_at = s.now
        set_status(s, out, "error")
    end
end

local function on_restored(s, event, out)
    local at_start = s.restoring
    s.restoring = false
    if event.ok then
        s.saved, s.seen = event.stamp, event.stamp
        s.dirty_since = nil
        set_status(s, out, "restored")
    elseif not at_start then
        set_status(s, out, "no backup")
    end
end

local function on_command(s, id, out)
    if id == "save_now" then
        save(s, out)
    elseif id == "restore" then
        out[#out + 1] = { kind = "document.restore" }
    elseif id == "toggle" then
        s.enabled = not s.enabled
        set_status(s, out, s.enabled and "on" or "off")
    end
end

function P.handle(s, event, ctx)
    local out = array()
    local name = event.name
    if name == "autosave.tick" then
        on_tick(s, event, out)
    elseif name == "document.saved" then
        on_saved(s, event, out)
    elseif name == "document.restored" then
        on_restored(s, event, out)
    elseif name == "app.exit" then
        if s.enabled and s.started and event.stamp ~= s.saved then
            out[#out + 1] = { kind = "document.save" }
        end
    elseif name == "command" then
        on_command(s, event.id, out)
    end
    return out
end

function P.deactivate(s) end

return P