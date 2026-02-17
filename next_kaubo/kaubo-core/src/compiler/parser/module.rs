use super::stmt::Stmt;

#[derive(Debug, Clone)]
pub struct ModuleKind {
    pub statements: Vec<Stmt>,
}

pub type Module = Box<ModuleKind>;
