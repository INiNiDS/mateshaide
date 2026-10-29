fn activate() {
    this.current = "";
    this.caret = 0;
    this.anchor = 0;
    this.dragging = false;
    this.preedit = "";
    this.size = 26.0;
    #{
        tools: [#{ id: "text", label: "Text", text_input: true }],
        object_types: ["block"],
        services: ["insert.v1"],
        commands: []
    }
}

fn find_node(nodes, id) {
    for n in nodes { if n.id == id { return n; } }
    #{}
}

fn clamp_index(i, len) {
    if i < 0 { 0 } else if i > len { len } else { i }
}

fn geometry(n) {
    let l = text_layout(n.data.text, n.data.size);
    n.width = if l.width < 24.0 { 24.0 } else { l.width + 12.0 };
    n.height = l.height;
    n
}

fn new_node(ctx, x, y, text) {
    let n = geometry(#{
        id: ctx.new_id,
        "type": "block",
        schema_version: 1,
        x: x, y: y, width: 24.0, height: this.size * 1.25,
        data: #{ text: text, size: this.size, color: [0.92, 0.94, 0.98, 1.0] }
    });
    this.current = n.id;
    this.caret = graphemes(text).len();
    this.anchor = this.caret;
    n
}

// The range replacement and caret policy are implemented here, not in Rust.
fn replace_selection(n, value) {
    let a = graphemes(n.data.text);
    let lo = clamp_index(min(this.caret, this.anchor), a.len());
    let hi = clamp_index(max(this.caret, this.anchor), a.len());
    let normalized = value;
    normalized.replace("\r\n", "\n");
    normalized.replace("\r", "\n");
    let inserted = graphemes(normalized);
    let result = [];
    for i in 0..lo { result.push(a[i]); }
    for g in inserted { result.push(g); }
    for i in hi..a.len() { result.push(a[i]); }
    n.data.text = join_text(result);
    this.caret = lo + inserted.len();
    this.anchor = this.caret;
    this.preedit = "";
    geometry(n)
}

fn selected_text(n) {
    let a = graphemes(n.data.text);
    let lo = clamp_index(min(this.caret, this.anchor), a.len());
    let hi = clamp_index(max(this.caret, this.anchor), a.len());
    let out = [];
    for i in lo..hi { out.push(a[i]); }
    join_text(out)
}

fn index_at(n, x, y) {
    let layout = text_layout(n.data.text, n.data.size);
    let row = clamp_index(((n.y - y) / layout.line_height).floor().to_int(), layout.lines.len() - 1);
    let best = 0;
    let distance = 1.0e30;
    for i in 0..layout.carets.len() {
        let c = layout.carets[i];
        if c.line == row {
            let d = abs(c.x - (x - n.x));
            if d < distance { distance = d; best = i; }
        }
    }
    best
}

fn move_cursor(n, key, ctrl, shift) {
    let a = graphemes(n.data.text);
    this.caret = clamp_index(this.caret, a.len());
    this.anchor = clamp_index(this.anchor, a.len());
    let old = this.caret;
    if key == "ArrowLeft" {
        if !shift && this.anchor != this.caret { this.caret = min(this.anchor, this.caret); }
        else { this.caret = max(0, this.caret - 1); }
    } else if key == "ArrowRight" {
        if !shift && this.anchor != this.caret { this.caret = max(this.anchor, this.caret); }
        else { this.caret = min(a.len(), this.caret + 1); }
    } else {
        let l = text_layout(n.data.text, n.data.size);
        let c = l.carets[this.caret];
        if key == "Home" {
            if ctrl { this.caret = 0; } else { this.caret = l.lines[c.line].start; }
        } else if key == "End" {
            if ctrl { this.caret = a.len(); } else { this.caret = l.lines[c.line].end; }
        } else {
            let target = c.line + if key == "ArrowUp" { -1 } else { 1 };
            let best = old;
            let distance = 1.0e30;
            for i in 0..l.carets.len() {
                let p = l.carets[i];
                if p.line == target && abs(p.x - c.x) < distance { best = i; distance = abs(p.x - c.x); }
            }
            this.caret = best;
        }
    }
    if !shift { this.anchor = this.caret; }
}

fn handle(event, ctx) {
    if event.name == "blur" || event.name == "tool.selected" {
        this.preedit = "";
        this.dragging = false;
        return [#{kind: "input.blur"}];
    }
    if event.name == "document.changed" {
        let n = find_node(ctx.nodes, this.current);
        if n.len() == 0 { this.current = ""; this.caret = 0; this.anchor = 0; }
        else {
            let length = graphemes(n.data.text).len();
            this.caret = clamp_index(this.caret, length);
            this.anchor = clamp_index(this.anchor, length);
        }
        this.preedit = "";
        return [];
    }
    if event.name == "pointer" {
        if event.phase == "up" { this.dragging = false; return []; }
        if event.phase == "move" {
            if this.dragging {
                let n = find_node(ctx.nodes, this.current);
                if n.len() > 0 { this.caret = index_at(n, event.x, event.y); }
            }
            return [];
        }
        let hit = #{};
        let i = ctx.nodes.len();
        while i > 0 {
            i -= 1;
            let n = ctx.nodes[i];
            if n["type"] == "block" && event.x >= n.x && event.x <= n.x + n.width && event.y <= n.y && event.y >= n.y - n.height {
                hit = n; break;
            }
        }
        this.dragging = true;
        if hit.len() == 0 {
            let n = this.new_node(ctx, event.x, event.y, "");
            return [#{kind: "node.create", node: n}, #{kind: "input.focus"}];
        }
        let same = this.current == hit.id;
        this.current = hit.id;
        this.caret = index_at(hit, event.x, event.y);
        if !event.shift || !same { this.anchor = this.caret; }
        return [#{kind: "input.focus"}];
    }
    if event.name == "ime.preedit" { this.preedit = event.text; return []; }
    let n = find_node(ctx.nodes, this.current);
    if n.len() == 0 || !ctx.focused { return []; }
    if event.name == "text" || (event.name == "clipboard" && event.ok) {
        if event.text == "" { return []; }
        n = this.replace_selection(n, event.text);
        return [#{kind: "node.update", node: n}];
    }
    if event.name != "key" { return []; }
    let key = event.key;
    if event.ctrl {
        if key == "a" { this.anchor = 0; this.caret = graphemes(n.data.text).len(); return []; }
        if key == "v" { return [#{kind: "clipboard.read"}]; }
        if key == "c" || key == "x" {
            if this.caret == this.anchor { return []; }
            let result = [#{kind: "clipboard.write", text: this.selected_text(n)}];
            if key == "x" { n = this.replace_selection(n, ""); result.push(#{kind: "node.update", node: n}); }
            return result;
        }
    }
    if key == "Escape" { this.preedit = ""; this.dragging = false; return [#{kind: "input.blur"}]; }
    if key == "ArrowLeft" || key == "ArrowRight" || key == "ArrowUp" || key == "ArrowDown" || key == "Home" || key == "End" {
        this.move_cursor(n, key, event.ctrl, event.shift); return [];
    }
    if key == "Backspace" || key == "Delete" {
        let length = graphemes(n.data.text).len();
        this.caret = clamp_index(this.caret, length);
        this.anchor = clamp_index(this.anchor, length);
        if this.caret == this.anchor {
            if key == "Backspace" { this.anchor = max(0, this.caret - 1); }
            else { this.anchor = min(length, this.caret + 1); }
        }
        if this.caret == this.anchor { return []; }
        n = this.replace_selection(n, ""); return [#{kind: "node.update", node: n}];
    }
    if key == "Enter" || key == "Tab" {
        n = this.replace_selection(n, if key == "Enter" { "\n" } else { "\t" });
        return [#{kind: "node.update", node: n}];
    }
    []
}

fn render(node, ctx) {
    [#{kind: "text", x: node.x, y: node.y, text: node.data.text, size: node.data.size, color: node.data.color}]
}

fn overlay(ctx) {
    if !ctx.focused { return []; }
    let n = find_node(ctx.nodes, this.current);
    if n.len() == 0 { return []; }
    let l = text_layout(n.data.text, n.data.size);
    let caret = clamp_index(this.caret, l.carets.len() - 1);
    let lo = clamp_index(min(this.anchor, this.caret), l.carets.len() - 1);
    let hi = clamp_index(max(this.anchor, this.caret), l.carets.len() - 1);
    let out = [];
    if lo != hi {
        for row in l.lines {
            let a = max(lo, row.start);
            let b = min(hi, row.end);
            if a <= b && lo <= row.end && hi >= row.start {
                let x1 = l.carets[a].x;
                let x2 = if b == row.end { row.width } else { l.carets[b].x };
                out.push(#{kind: "rect", x: n.x + min(x1, x2), y: n.y - row.y, width: max(abs(x2-x1), 3.0), height: l.line_height, color: [0.22,0.50,1.0,0.30]});
            }
        }
    }
    let c = l.carets[caret];
    out.push(#{kind: "rect", x: n.x + c.x, y: n.y - c.y, width: 1.5 / ctx.zoom, height: l.line_height, color: [0.6,0.8,1.0,1.0]});
    if this.preedit != "" {
        out.push(#{kind: "text", x: n.x+c.x, y: n.y-c.y, text: this.preedit, size: n.data.size, color: [0.6,0.8,1.0,1.0]});
        let m = text_layout(this.preedit, n.data.size);
        out.push(#{kind: "line", x1:n.x+c.x, y1:n.y-c.y-l.line_height, x2:n.x+c.x+m.width, y2:n.y-c.y-l.line_height, width:1.0/ctx.zoom, color:[0.6,0.8,1.0,1.0]});
    }
    out
}

fn service(name, args, ctx) {
    if name != "insert.v1" { throw "Unknown service"; }
    let n = this.new_node(ctx, args.x, args.y, args.text);
    #{value: #{id:n.id}, commands: [#{kind: "node.create", node:n}]}
}
fn deactivate() {}
