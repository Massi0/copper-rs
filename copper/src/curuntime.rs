use crate::common::CuListsManager;
use crate::config::{CuConfig, NodeId};
use crate::config::{Node, NodeInstanceConfig};
use crate::CuResult;

// CT is a tuple of all the tasks
// CL is the type of the copper list
pub struct CuRuntime<CT, CL: Sized + PartialEq> {
    task_instances: CT,
    copper_lists: CuListsManager<CL, 10>,
}

impl<CT, CL: Sized + PartialEq> CuRuntime<CT, CL> {
    pub fn new(
        config: &CuConfig,
        tasks_instanciator: impl Fn(Vec<Option<&NodeInstanceConfig>>) -> CuResult<CT>,
    ) -> CuResult<Self> {
        let all_instances_configs: Vec<Option<&NodeInstanceConfig>> = config
            .get_all_nodes()
            .iter()
            .map(|node_config| node_config.get_instance_config())
            .collect();
        let task_instances = tasks_instanciator(all_instances_configs)?;
        Ok(Self {
            task_instances,
            copper_lists: CuListsManager::new(),
        })
    }
}
use petgraph::algo::toposort;
pub fn compute_runtime_plan(config: &CuConfig) -> CuResult<Vec<(NodeId, &Node)>> {
    let sorted_nodes = toposort(&config.graph, None).expect("Cycle detected in the graph");
    let result = sorted_nodes
        .iter()
        .map(|node| {
            let id = node.index() as NodeId;
            let node = config.get_node(id).unwrap();
            (id, node)
        })
        .collect();
    Ok(result)
}

//tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Node;
    use crate::cutask::{CuMsg, CuSrcTask};
    use crate::cutask::{CuSinkTask, CuTaskLifecycle};
    pub struct TestSource {}

    impl CuTaskLifecycle for TestSource {
        fn new(config: Option<&NodeInstanceConfig>) -> CuResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }
    }

    impl CuSrcTask for TestSource {
        type Payload = ();
        fn process(&mut self, empty_msg: &mut CuMsg<Self::Payload>) -> CuResult<()> {
            Ok(())
        }
    }

    pub struct TestSink {}

    impl CuTaskLifecycle for TestSink {
        fn new(config: Option<&NodeInstanceConfig>) -> CuResult<Self>
        where
            Self: Sized,
        {
            Ok(Self {})
        }
    }

    impl CuSinkTask for TestSink {
        type Input = ();

        fn process(&mut self, input: &CuMsg<Self::Input>) -> CuResult<()> {
            Ok(())
        }
    }

    // Those should be generated by the derive macro
    type Tasks = (TestSource, TestSink);
    type Msgs = ((),);

    fn tasks_instanciator(
        all_instances_configs: Vec<Option<&NodeInstanceConfig>>,
    ) -> CuResult<Tasks> {
        Ok((
            TestSource::new(all_instances_configs[0])?,
            TestSink::new(all_instances_configs[1])?,
        ))
    }

    #[test]
    fn test_runtime_instanciation() {
        let mut config = CuConfig::default();
        config.add_node(Node::new("a", "TestSource"));
        config.add_node(Node::new("b", "TestSink"));
        config.connect(0, 1, "()");
        let runtime = CuRuntime::<Tasks, Msgs>::new(&config, tasks_instanciator);
        assert!(runtime.is_ok());
    }
}
