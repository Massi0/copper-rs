use std::collections::HashMap;
use std::fs::read_to_string;

use crate::{CuError, CuResult};
use petgraph::dot::Config as PetConfig;
use petgraph::dot::Dot;
use petgraph::stable_graph::{EdgeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use ron::extensions::Extensions;
use ron::value::Value as RonValue;
use ron::Options;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uom::si::rational::Time;
use uom::si::time::nanosecond;

pub type NodeId = u32;
pub type NodeInstanceConfig = HashMap<String, Value>;
pub type Edge = (NodeId, NodeId, String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Value(RonValue);

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value(RonValue::Number(value.into()))
    }
}
impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value(RonValue::Number(value.into()))
    }
}

impl From<Value> for i32 {
    fn from(value: Value) -> Self {
        if let RonValue::Number(num) = value.0 {
            if let Some(i) = num.as_i64() {
                i as i32
            } else {
                panic!("Expected an integer value")
            }
        } else {
            panic!("Expected a Number variant")
        }
    }
}

impl From<Value> for f64 {
    fn from(value: Value) -> Self {
        if let RonValue::Number(num) = value.0 {
            if let Some(f) = num.as_f64() {
                f
            } else {
                panic!("Expected a float value")
            }
        } else {
            panic!("Expected a Number variant")
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value(RonValue::String(value))
    }
}

impl From<Value> for String {
    fn from(value: Value) -> Self {
        if let RonValue::String(s) = value.0 {
            s
        } else {
            panic!("Expected a String variant")
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_period_ns: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<NodeInstanceConfig>,
}

impl Node {
    pub fn new(id: &str, ptype: &str) -> Self {
        Node {
            id: id.to_string(),
            type_: Some(ptype.to_string()),
            base_period_ns: None,
            config: None,
        }
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    #[allow(dead_code)]
    pub fn set_type(mut self, name: Option<String>) -> Self {
        self.type_ = name;
        self
    }

    pub fn get_type(&self) -> &str {
        self.type_.as_ref().unwrap()
    }

    pub fn get_instance_config(&self) -> Option<&NodeInstanceConfig> {
        self.config.as_ref()
    }

    #[allow(dead_code)]
    pub fn base_period(&self) -> Option<Time> {
        self.base_period_ns
            .map(|frequency| Time::new::<nanosecond>(frequency.into()))
    }

    pub fn set_base_period(mut self, period: Time) -> Self {
        let as_ns = &period.get::<nanosecond>();
        debug_assert_eq!(*as_ns.denom(), 1isize); // We normalize to the ns
        self.base_period_ns = Some(*as_ns.numer());
        self
    }

    pub fn get_param<T: From<Value>>(&self, key: &str) -> Option<T> {
        let pc = self.config.as_ref()?;
        let v = pc.get(key)?;
        Some(T::from(v.clone()))
    }

    pub fn set_param<T: Into<Value>>(&mut self, key: &str, value: T) {
        if self.config.is_none() {
            self.config = Some(HashMap::new());
        }
        self.config
            .as_mut()
            .unwrap()
            .insert(key.to_string(), value.into());
    }
}

// Serialization format
// (
//   tasks : [ (id: "toto", type: "zorglub::MyType", config: {...}),
//             (id: "titi", type: "zorglub::MyType2", config: {...})]
//   cnx : [ (src: "toto", dst: "titi", msg: "zorglub::MyMsgType"),...]
// )

#[derive(Serialize, Deserialize, Debug)]
pub struct Cnx {
    src: String,
    dst: String,
    msg: String,
}

#[derive(Debug)]
pub struct CuConfig {
    // This is not what is directly serialized, see the custom serialization below.
    pub graph: StableDiGraph<Node, String, NodeId>,
}

#[derive(Serialize, Deserialize, Default)]
struct CuConfigRepresentation {
    tasks: Vec<Node>,
    cnx: Vec<Cnx>,
}

impl<'de> Deserialize<'de> for CuConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let representation = CuConfigRepresentation::deserialize(deserializer)
            .expect("Failed to deserialize config");

        let mut cuconfig = CuConfig::default();
        for task in representation.tasks {
            cuconfig.add_node(task);
        }

        for c in representation.cnx {
            let src = cuconfig
                .graph
                .node_indices()
                .find(|i| cuconfig.graph[*i].id == c.src)
                .expect("Source node not found");
            let dst = cuconfig
                .graph
                .node_indices()
                .find(|i| cuconfig.graph[*i].id == c.dst)
                .expect("Destination node not found");
            cuconfig.connect(src.index() as NodeId, dst.index() as NodeId, &c.msg);
        }

        Ok(cuconfig)
    }
}

impl Serialize for CuConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tasks: Vec<Node> = self
            .graph
            .node_indices()
            .map(|idx| self.graph[idx].clone())
            .collect();

        let cnx: Vec<Cnx> = self
            .graph
            .edge_indices()
            .map(|edge| {
                let (source, target) = self.graph.edge_endpoints(edge).unwrap();
                (
                    self.graph[source].id.clone(),
                    self.graph[target].id.clone(),
                    self.graph[edge].clone(),
                )
            })
            .map(|(src, dst, msg)| Cnx { src, dst, msg })
            .collect();

        CuConfigRepresentation { tasks, cnx }.serialize(serializer)
    }
}

impl Default for CuConfig {
    fn default() -> Self {
        CuConfig {
            graph: StableDiGraph::new(),
        }
    }
}

impl CuConfig {
    pub fn add_node(&mut self, node: Node) -> NodeId {
        self.graph.add_node(node).index() as NodeId
    }

    pub fn get_node(&self, node_id: NodeId) -> Option<&Node> {
        self.graph.node_weight(node_id.into())
    }

    pub fn get_src_edges(&self, node_id: NodeId) -> Vec<usize> {
        self.graph
            .edges_directed(node_id.into(), petgraph::Direction::Outgoing)
            .map(|edge| edge.id().index())
            .collect()
    }
    pub fn get_dst_edges(&self, node_id: NodeId) -> Vec<usize> {
        self.graph
            .edges_directed(node_id.into(), petgraph::Direction::Incoming)
            .map(|edge| edge.id().index())
            .collect()
    }
    pub fn get_edge_weight(&self, index: usize) -> Option<String> {
        self.graph
            .edge_weight(EdgeIndex::new(index))
            .map(|s| s.clone())
    }

    pub fn get_all_nodes(&self) -> Vec<&Node> {
        self.graph
            .node_indices()
            .map(|node| &self.graph[node])
            .collect()
    }

    pub fn get_all_edges(&self) -> Vec<Edge> {
        self.graph
            .edge_indices()
            .map(|edge| {
                let (source, target) = self.graph.edge_endpoints(edge).unwrap();
                (
                    source.index() as NodeId,
                    target.index() as NodeId,
                    self.graph[edge].clone(),
                )
            })
            .collect()
    }

    pub fn connect(&mut self, source: NodeId, target: NodeId, msg_type: &str) {
        self.graph
            .add_edge(source.into(), target.into(), msg_type.to_string());
    }

    pub fn get_options() -> Options {
        Options::default()
            .with_default_extension(Extensions::IMPLICIT_SOME)
            .with_default_extension(Extensions::UNWRAP_NEWTYPES)
            .with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES)
    }

    pub fn serialize_ron(&self) -> String {
        let ron = Self::get_options();
        let pretty = ron::ser::PrettyConfig::default();
        ron.to_string_pretty(&self, pretty).unwrap()
    }

    pub fn deserialize_ron(ron: &str) -> Self {
        Self::get_options()
            .from_str(ron)
            .expect("Syntax Error in config")
    }

    pub fn render(&self, output: &mut dyn std::io::Write) {
        let dot = Dot::with_config(&self.graph, &[PetConfig::EdgeNoLabel]);
        write!(output, "{:?}", dot).unwrap();
    }

    pub fn get_all_instances_configs(&self) -> Vec<Option<&NodeInstanceConfig>> {
        self.get_all_nodes()
            .iter()
            .map(|node_config| node_config.get_instance_config())
            .collect()
    }
}

pub fn read_configuration(config_filename: &str) -> CuResult<CuConfig> {
    let config_content = read_to_string(config_filename).map_err(|e| {
        CuError::from(format!(
            "Failed to read configuration file: {:?}",
            &config_filename
        ))
        .add_context(e.to_string().as_str())
    })?;
    Ok(CuConfig::deserialize_ron(&config_content))
}

// tests
#[cfg(test)]
mod tests {
    use uom::si::power::watt;
    use uom::si::time::millisecond;
    use uom::si::time::second;

    use super::*;

    #[test]
    fn test_base_period() {
        let node = Node {
            id: "test".to_string(),
            type_: Some("test".to_string()),
            config: Some(HashMap::new()),
            base_period_ns: Some(1_000_000_000),
        };
        assert_eq!(node.base_period(), Some(Time::new::<second>(1.into())));

        let node = Node::new("test2", "package::Plugin")
            .set_base_period(Time::new::<millisecond>(500.into()));
        assert_eq!(node.base_period_ns, Some(500_000_000));
        assert_eq!(
            node.base_period(),
            Some(Time::new::<nanosecond>(500_000_000.into()))
        );
    }

    #[test]
    fn test_plain_serialize() {
        let mut config = CuConfig::default();
        let n1 = config.add_node(Node::new("test1", "package::Plugin1"));
        let n2 = config.add_node(Node::new("test2", "package::Plugin2"));
        config.connect(n1, n2, "msgpkg::MsgType");
        let serialized = config.serialize_ron();
        let deserialized = CuConfig::deserialize_ron(&serialized);
        assert_eq!(config.graph.node_count(), deserialized.graph.node_count());
        assert_eq!(config.graph.edge_count(), deserialized.graph.edge_count());
    }

    #[test]
    fn test_serialize_with_params() {
        let mut config = CuConfig::default();
        let mut camera = Node::new("copper-camera", "camerapkg::Camera")
            .set_base_period(Time::new::<second>(60.into()));
        camera.set_param::<Value>("resolution-height", 1080.into());
        config.add_node(camera);
        let serialized = config.serialize_ron();
        println!("{}", serialized);
        let deserialized = CuConfig::deserialize_ron(&serialized);
        assert_eq!(
            deserialized
                .get_node(0)
                .unwrap()
                .get_param::<i32>("resolution-height")
                .unwrap(),
            1080
        );
    }
}
