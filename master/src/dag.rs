//! Análisis de DAG y generación de tareas
//! Procesa la estructura DAG de la especificación del job y crea tareas

use common::{Dag, DagNode};
use std::collections::{HashMap, HashSet};

/// Representa una etapa de tarea en la ejecución del DAG
#[derive(Debug, Clone)]
pub struct TaskStage {
    pub node_id: String,
    pub operation: String,
    pub fn_name: Option<String>,
    pub key: Option<String>,
    pub path: Option<String>,
    pub partitions: Option<usize>,
    pub dependencies: Vec<String>, // IDs de nodos de los que esta etapa depende
}

/// Analizar DAG y crear etapas de tareas
pub fn parse_dag(dag: &Dag) -> Result<Vec<TaskStage>, String> {
    // Construir grafo de dependencias
    let node_map: HashMap<String, &DagNode> = dag
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect();

    let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

    // Inicializar dependencias
    for node in &dag.nodes {
        dependencies.insert(node.id.clone(), Vec::new());
        dependents.insert(node.id.clone(), Vec::new());
    }

    // Procesar aristas para construir grafo de dependencias
    for edge in &dag.edges {
        let from = &edge.0;
        let to = &edge.1;

        if !node_map.contains_key(from) {
            return Err(format!("Arista referencia nodo desconocido: {}", from));
        }
        if !node_map.contains_key(to) {
            return Err(format!("Arista referencia nodo desconocido: {}", to));
        }

        dependencies.get_mut(to).unwrap().push(from.clone());
        dependents.get_mut(from).unwrap().push(to.clone());
    }

    // Ordenamiento topológico para determinar orden de ejecución
    let mut stages = Vec::new();
    let mut visited = HashSet::new();
    let mut in_progress = HashSet::new();

    for node in &dag.nodes {
        if !visited.contains(&node.id) {
            topological_sort(
                &node.id,
                &node_map,
                &dependencies,
                &mut visited,
                &mut in_progress,
                &mut stages,
            )?;
        }
    }

    Ok(stages)
}

/// Ayudante de ordenamiento topológico
fn topological_sort(
    node_id: &str,
    node_map: &HashMap<String, &DagNode>,
    dependencies: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    in_progress: &mut HashSet<String>,
    stages: &mut Vec<TaskStage>,
) -> Result<(), String> {
    if in_progress.contains(node_id) {
        return Err(format!("Ciclo detectado en DAG en el nodo: {}", node_id));
    }

    if visited.contains(node_id) {
        return Ok(());
    }

    in_progress.insert(node_id.to_string());

    // Procesar dependencias primero
    if let Some(deps) = dependencies.get(node_id) {
        for dep_id in deps {
            topological_sort(dep_id, node_map, dependencies, visited, in_progress, stages)?;
        }
    }

    in_progress.remove(node_id);
    visited.insert(node_id.to_string());

    // Crear etapa para este nodo
    let node = node_map
        .get(node_id)
        .ok_or_else(|| format!("Nodo no encontrado: {}", node_id))?;

    let stage = TaskStage {
        node_id: node.id.clone(),
        operation: node.op.clone(),
        fn_name: node.fn_name.clone(),
        key: node.key.clone(),
        path: node.path.clone(),
        partitions: node.partitions,
        dependencies: dependencies
            .get(node_id)
            .cloned()
            .unwrap_or_default(),
    };

    stages.push(stage);
    Ok(())
}

/// Obtener nodos raíz (nodos sin dependencias)
pub fn get_root_nodes(dag: &Dag) -> Vec<String> {
    let mut has_dependencies: HashSet<String> = HashSet::new();

    for edge in &dag.edges {
        has_dependencies.insert(edge.1.clone());
    }

    dag.nodes
        .iter()
        .filter(|node| !has_dependencies.contains(&node.id))
        .map(|node| node.id.clone())
        .collect()
}

/// Obtener nodos hoja (nodos sin dependientes)
pub fn get_leaf_nodes(dag: &Dag) -> Vec<String> {
    let mut has_dependents: HashSet<String> = HashSet::new();

    for edge in &dag.edges {
        has_dependents.insert(edge.0.clone());
    }

    dag.nodes
        .iter()
        .filter(|node| !has_dependents.contains(&node.id))
        .map(|node| node.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_dag() {
        let dag = Dag {
            nodes: vec![
                DagNode {
                    id: "read".to_string(),
                    op: "read_csv".to_string(),
                    path: Some("data.csv".to_string()),
                    fn_name: None,
                    key: None,
                    partitions: Some(4),
                },
                DagNode {
                    id: "map".to_string(),
                    op: "map".to_string(),
                    path: None,
                    fn_name: Some("to_lower".to_string()),
                    key: None,
                    partitions: None,
                },
            ],
            edges: vec![DagEdge("read".to_string(), "map".to_string())],
        };

        let stages = parse_dag(&dag).unwrap();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].node_id, "read");
        assert_eq!(stages[1].node_id, "map");
    }

    #[test]
    fn test_detect_cycle() {
        let dag = Dag {
            nodes: vec![
                DagNode {
                    id: "a".to_string(),
                    op: "map".to_string(),
                    path: None,
                    fn_name: None,
                    key: None,
                    partitions: None,
                },
                DagNode {
                    id: "b".to_string(),
                    op: "map".to_string(),
                    path: None,
                    fn_name: None,
                    key: None,
                    partitions: None,
                },
            ],
            edges: vec![
                DagEdge("a".to_string(), "b".to_string()),
                DagEdge("b".to_string(), "a".to_string()), // ¡Ciclo!
            ],
        };

        assert!(parse_dag(&dag).is_err());
    }
}

