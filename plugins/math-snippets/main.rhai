fn activate() {
    # {
        tools: [# {
                    id: "formula",
                    label: "Formula",
                    text_input: false
                }],
        services: []
    }
}
fn handle(event) {
    if event.name == "pointer" {
        [# {
             kind: "call",
             plugin: "org.matesha.text",
             service: "insert.v1",
             args: # {
                 x: event.x,
                 y: event.y,
                 text: "x² + y² = r²"
             }
         }]
    } else {
        []
    }
}
fn deactivate() {}