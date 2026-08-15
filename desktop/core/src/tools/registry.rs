use super::{BashTool, EditFileTool, GlobTool, GrepTool, ListDirTool, ReadFileTool, Tool, WriteFileTool};
use std::collections::HashMap;
use std::sync::Arc;

pub fn all_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ReadFileTool),
        Arc::new(WriteFileTool),
        Arc::new(EditFileTool),
        Arc::new(ListDirTool),
        Arc::new(GlobTool),
        Arc::new(GrepTool),
        Arc::new(BashTool),
    ]
}

pub fn tools_by_name() -> HashMap<String, Arc<dyn Tool>> {
    all_tools().into_iter().map(|t| (t.definition().name.clone(), t)).collect()
}
