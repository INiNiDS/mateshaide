use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginNode {
    pub id: String,
    #[serde(default)]
    pub owner: String,
    #[serde(rename = "type")]
    pub object_type: String,
    #[serde(default = "schema_one")]
    pub schema_version: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub data: Value,
}
fn schema_one() -> u32 {
    1
}
impl PluginNode {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.id.len() <= 192 && !self.id.is_empty(),
            "invalid node ID"
        );
        ensure!(
            super::manifest::valid_id(&self.owner) && super::manifest::valid_id(&self.object_type),
            "invalid node type/owner"
        );
        ensure!(
            [self.x, self.y, self.width, self.height]
                .iter()
                .all(|n| n.is_finite() && n.abs() <= 1.0e8),
            "invalid node geometry"
        );
        ensure!(
            self.width >= 0.0 && self.height >= 0.0 && self.schema_version > 0,
            "invalid node bounds/schema"
        );
        ensure!(
            serde_json::to_vec(&self.data)?.len() <= 262144,
            "node data too large"
        );
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedDocument {
    format: u32,
    instances: Vec<crate::ShapeInstance>,
    texts: Vec<crate::CanvasText>,
    nodes: Vec<PluginNode>,
}
impl crate::Canvas {
    pub fn document_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(&SavedDocument {
            format: 1,
            instances: self.instances.clone(),
            texts: self.texts.clone(),
            nodes: self.plugin_nodes.clone(),
        })?)
    }
    pub fn load_document_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(bytes.len() <= 32 * 1024 * 1024, "document too large");
        let doc: SavedDocument = serde_json::from_slice(bytes)?;
        ensure!(doc.format == 1, "unsupported document format");
        ensure!(
            doc.nodes.len() <= 5000 && doc.instances.len() <= 250000 && doc.texts.len() <= 20000,
            "document object limit"
        );
        let mut ids = std::collections::BTreeSet::new();
        for node in &doc.nodes {
            node.validate()?;
            ensure!(ids.insert(&node.id), "duplicate node ID");
        }
        let valid_color = |c: crate::Color| {
            c.to_array()
                .iter()
                .all(|n| n.is_finite() && (0.0..=1.0).contains(n))
        };
        for t in &doc.texts {
            ensure!(
                t.position.is_finite()
                    && t.size.is_finite()
                    && t.size > 0.0
                    && t.size <= 100000.0
                    && t.text.len() <= 262144
                    && valid_color(t.color),
                "invalid legacy text"
            );
        }
        for s in &doc.instances {
            ensure!(
                s.center
                    .iter()
                    .chain(&s.size)
                    .chain(&s.a)
                    .chain(&s.b)
                    .all(|v| v.is_finite() && v.abs() <= 1.0e8)
                    && valid_color(s.color),
                "invalid shape geometry"
            );
        }
        self.record_history();
        self.instances = doc.instances;
        self.texts = doc.texts;
        self.plugin_nodes = doc.nodes;
        Ok(())
    }
}
