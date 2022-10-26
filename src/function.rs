use std::fmt::Display;

use crate::{bytecode::Chunk, object::Handle, string::String};

#[derive(Clone, Debug)]
pub struct Function {
    arity: usize,
    chunk: Option<Chunk>,
    name: Option<String>,
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match &self.name {
            Some(name) => name.as_str(),
            None => "<script>",
        };
        write!(f, "{}", name)
    }
}

impl Function {
    pub(crate) fn new(chunk: Option<Chunk>, name: Option<String>) -> Self {
        Self {
            arity: 0,
            chunk,
            name,
        }
    }

    pub(crate) fn name(&self) -> &str {
        match &self.name {
            Some(name) => name.as_str(),
            None => "<script>",
        }
    }

    pub(crate) fn chunk(&self) -> Option<&Chunk> {
        self.chunk.as_ref()
    }
}
