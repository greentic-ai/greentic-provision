use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackManifest {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub meta: PackMeta,
    #[serde(default)]
    pub flows: Vec<PackFlow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PackMeta {
    #[serde(default)]
    pub entry_flows: EntryFlows,
    #[serde(default)]
    pub requires_public_base_url: bool,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(untagged)]
pub enum EntryFlows {
    #[default]
    Empty,
    Map(std::collections::BTreeMap<String, String>),
    List(Vec<EntryFlowDescriptor>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntryFlowDescriptor {
    pub entry: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub flow_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackFlow {
    pub entry: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ProvisionDescriptor {
    pub pack_id: String,
    pub pack_version: String,
    pub setup_entry_flow: String,
    pub requirements_flow: Option<String>,
    pub subscriptions_flow: Option<String>,
    pub requires_public_base_url: bool,
    pub outputs: Vec<String>,
}

pub trait ProvisionPackDiscovery {
    fn discover(pack: &PackManifest) -> Option<ProvisionDescriptor>;
}

pub struct DefaultProvisionPackDiscovery;

impl ProvisionPackDiscovery for DefaultProvisionPackDiscovery {
    fn discover(pack: &PackManifest) -> Option<ProvisionDescriptor> {
        let setup_entry_flow = entry_flow_id(pack, "setup")?;
        let requirements_flow = entry_flow_id(pack, "requirements");
        let subscriptions_flow = entry_flow_id(pack, "subscriptions");

        Some(ProvisionDescriptor {
            pack_id: pack.id.clone(),
            pack_version: pack.version.clone(),
            setup_entry_flow,
            requirements_flow,
            subscriptions_flow,
            requires_public_base_url: pack.meta.requires_public_base_url,
            outputs: pack.meta.capabilities.clone(),
        })
    }
}

fn entry_flow_id(pack: &PackManifest, entry_name: &str) -> Option<String> {
    if let Some(entry_flow) = entry_flow_from_meta(&pack.meta.entry_flows, entry_name) {
        return Some(entry_flow);
    }

    for flow in &pack.flows {
        let entry = flow.entry.as_deref().or(flow.name.as_deref());
        if entry == Some(entry_name)
            && let Some(id) = flow.id.clone().or(flow.name.clone())
        {
            return Some(id);
        }
    }

    None
}

fn entry_flow_from_meta(entry_flows: &EntryFlows, entry_name: &str) -> Option<String> {
    match entry_flows {
        EntryFlows::Empty => None,
        EntryFlows::Map(map) => map.get(entry_name).cloned(),
        EntryFlows::List(list) => list.iter().find_map(|flow| {
            let entry = flow.entry.as_deref().or(flow.name.as_deref());
            if entry == Some(entry_name) {
                flow.id
                    .clone()
                    .or_else(|| flow.flow_id.clone())
                    .or_else(|| flow.name.clone())
            } else {
                None
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_from_value(value: serde_json::Value) -> PackManifest {
        serde_json::from_value(value).expect("failed to deserialize PackManifest")
    }

    #[test]
    fn discover_uses_entry_flows_setup() {
        let manifest = manifest_from_value(serde_json::json!({
            "id": "pack-1",
            "version": "1.0.0",
            "meta": {
                "entry_flows": {
                    "setup": "setup-flow"
                }
            },
            "flows": []
        }));

        let descriptor =
            DefaultProvisionPackDiscovery::discover(&manifest).expect("missing descriptor");
        assert_eq!(descriptor.setup_entry_flow, "setup-flow");
        assert_eq!(descriptor.pack_id, "pack-1");
    }

    #[test]
    fn discover_returns_none_without_setup() {
        let manifest = manifest_from_value(serde_json::json!({
            "id": "pack-2",
            "version": "1.0.0",
            "meta": { "entry_flows": {} },
            "flows": []
        }));

        let descriptor = DefaultProvisionPackDiscovery::discover(&manifest);
        assert!(descriptor.is_none());
    }
}
