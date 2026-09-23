//! Pure response → attack-graph mapping (FR-D). Kept free of the AWS SDK so it can be unit-tested
//! offline; the adapter's `ResourceEnumerator` produces the intermediate JSON this maps.

use akumo_domain::epistemic::Epistemic;
use akumo_domain::graph::{Assertion, GraphNode, NodeKind, Provenance};
use akumo_domain::ids::Timestamp;
use akumo_domain::seam::RawResponse;

/// Map a raw enumeration response (`{ "items": [ { arn, name, type }, … ] }`) into attack-core
/// assertions. Users and roles both become `Principal` nodes; the IAM type is kept as inventory.
pub fn map_response(response: &RawResponse) -> Vec<Assertion> {
    let mut out = Vec::new();
    let Some(items) = response.raw.get("items").and_then(|v| v.as_array()) else {
        return out;
    };
    for item in items {
        let Some(arn) = item.get("arn").and_then(|v| v.as_str()) else {
            continue;
        };
        if arn.is_empty() {
            continue;
        }
        let mut attributes = serde_json::Map::new();
        if let Some(name) = item.get("name") {
            attributes.insert("name".to_string(), name.clone());
        }
        if let Some(iam_type) = item.get("type") {
            attributes.insert("iam_type".to_string(), iam_type.clone());
        }
        out.push(Assertion::Node(GraphNode {
            id: arn.to_string(),
            kind: NodeKind::Principal,
            attributes,
            epistemic: Epistemic::proven(),
            provenance: Provenance::new("aws.iam", Timestamp::from_millis(0)),
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_users_and_roles_to_principals() {
        let response = RawResponse {
            raw: serde_json::json!({
                "kind": "principals",
                "items": [
                    { "arn": "arn:aws:iam::1:user/alice", "name": "alice", "type": "user" },
                    { "arn": "arn:aws:iam::1:role/admin", "name": "admin", "type": "role" },
                    { "name": "no-arn" }
                ]
            }),
        };
        let assertions = map_response(&response);
        assert_eq!(assertions.len(), 2); // the arn-less item is skipped
        match &assertions[0] {
            Assertion::Node(node) => {
                assert_eq!(node.id, "arn:aws:iam::1:user/alice");
                assert_eq!(node.kind, NodeKind::Principal);
                assert_eq!(node.attributes.get("iam_type").and_then(|v| v.as_str()), Some("user"));
            }
            _ => panic!("expected a node"),
        }
    }

    #[test]
    fn empty_response_maps_to_nothing() {
        let response = RawResponse { raw: serde_json::json!({}) };
        assert!(map_response(&response).is_empty());
    }
}
