//! Runtime context and thread-safe lock-free arenas for `cat-core`.
//!
//! Stores deduplicated hash-consed descriptors, scalar atoms, and hypergraphs.
//! Provides ambient thread-local access (`ACTIVE_CTX`) for zero-overhead operator overloading.

use crate::descriptor::ObjectDescriptor;
use crate::handles::{DiagramId, ObjId, ScalarId, SymDimId};
use crate::hypergraph::OpenHypergraph;
use crate::scalar::ScalarAtom;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::cell::RefCell;
use std::sync::Arc;

/// Lock-free, sharded arena for Layer 0 ObjectDescriptors.
#[derive(Debug, Default)]
pub struct ObjArena {
    descriptors: RwLock<Vec<ObjectDescriptor>>,
    dedup: DashMap<ObjectDescriptor, ObjId>,
}

impl ObjArena {
    pub fn new() -> Self {
        let arena = Self {
            descriptors: RwLock::new(Vec::new()),
            dedup: DashMap::new(),
        };
        // Pre-seed ObjId(0) as standard dimensionless scalar
        arena.intern(ObjectDescriptor::DIMENSIONLESS_SCALAR);
        arena
    }

    pub fn intern(&self, desc: ObjectDescriptor) -> ObjId {
        if let Some(id) = self.dedup.get(&desc) {
            return *id;
        }
        let mut guard = self.descriptors.write();
        if let Some(id) = self.dedup.get(&desc) {
            return *id;
        }
        let id = ObjId(guard.len() as u32);
        guard.push(desc.clone());
        self.dedup.insert(desc, id);
        id
    }

    pub fn get(&self, id: ObjId) -> Option<ObjectDescriptor> {
        let guard = self.descriptors.read();
        guard.get(id.0 as usize).cloned()
    }
}

/// Lock-free, sharded arena for Layer 1 ScalarAtoms.
#[derive(Debug, Default)]
pub struct ScalarArena {
    scalars: RwLock<Vec<ScalarAtom>>,
    dedup: DashMap<ScalarAtom, ScalarId>,
}

impl ScalarArena {
    pub fn new() -> Self {
        let arena = Self {
            scalars: RwLock::new(Vec::new()),
            dedup: DashMap::new(),
        };
        // Pre-seed ScalarId(0) = Zero, ScalarId(1) = One
        arena.intern(ScalarAtom::Zero);
        arena.intern(ScalarAtom::One);
        arena
    }

    pub fn intern(&self, atom: ScalarAtom) -> ScalarId {
        if let Some(id) = self.dedup.get(&atom) {
            return *id;
        }
        let mut guard = self.scalars.write();
        if let Some(id) = self.dedup.get(&atom) {
            return *id;
        }
        let id = ScalarId(guard.len() as u32);
        guard.push(atom.clone());
        self.dedup.insert(atom, id);
        id
    }

    pub fn get(&self, id: ScalarId) -> Option<ScalarAtom> {
        let guard = self.scalars.read();
        guard.get(id.0 as usize).cloned()
    }
}

/// Lock-free, sharded arena for Layer 2 OpenHypergraph diagrams.
#[derive(Debug, Default)]
pub struct HyperArena {
    diagrams: RwLock<Vec<OpenHypergraph>>,
    dedup: DashMap<OpenHypergraph, DiagramId>,
}

impl HyperArena {
    pub fn new() -> Self {
        let arena = Self {
            diagrams: RwLock::new(Vec::new()),
            dedup: DashMap::new(),
        };
        // Pre-seed DiagramId(0) = Empty diagram
        arena.intern(OpenHypergraph::new());
        arena
    }

    pub fn intern(&self, mut diag: OpenHypergraph) -> DiagramId {
        diag.canonicalize();
        if let Some(id) = self.dedup.get(&diag) {
            return *id;
        }
        let mut guard = self.diagrams.write();
        if let Some(id) = self.dedup.get(&diag) {
            return *id;
        }
        let id = DiagramId(guard.len() as u32);
        guard.push(diag.clone());
        self.dedup.insert(diag, id);
        id
    }

    pub fn get(&self, id: DiagramId) -> Option<OpenHypergraph> {
        let guard = self.diagrams.read();
        guard.get(id.0 as usize).cloned()
    }
}

/// Arena for symbolic dimension identifiers.
#[derive(Debug, Default)]
pub struct SymDimArena {
    names: RwLock<Vec<String>>,
    dedup: DashMap<String, SymDimId>,
}

impl SymDimArena {
    pub fn new() -> Self {
        Self {
            names: RwLock::new(Vec::new()),
            dedup: DashMap::new(),
        }
    }

    pub fn intern(&self, name: &str) -> SymDimId {
        if let Some(id) = self.dedup.get(name) {
            return *id;
        }
        let mut guard = self.names.write();
        if let Some(id) = self.dedup.get(name) {
            return *id;
        }
        let id = SymDimId(guard.len() as u32);
        guard.push(name.to_string());
        self.dedup.insert(name.to_string(), id);
        id
    }

    pub fn get(&self, id: SymDimId) -> Option<String> {
        let guard = self.names.read();
        guard.get(id.0 as usize).cloned()
    }
}

/// Central runtime context managing all categorical arenas.
#[derive(Debug, Default)]
pub struct CatRuntimeContext {
    pub obj_arena: ObjArena,
    pub scalar_arena: ScalarArena,
    pub hyper_arena: HyperArena,
    pub sym_dim_arena: SymDimArena,
}

impl CatRuntimeContext {
    /// Create a new initialized runtime context.
    pub fn new() -> Self {
        Self {
            obj_arena: ObjArena::new(),
            scalar_arena: ScalarArena::new(),
            hyper_arena: HyperArena::new(),
            sym_dim_arena: SymDimArena::new(),
        }
    }

    /// Convenience helper to intern an object descriptor.
    pub fn intern_obj(&self, desc: ObjectDescriptor) -> ObjId {
        self.obj_arena.intern(desc)
    }

    /// Convenience helper to retrieve an object descriptor.
    pub fn get_obj(&self, id: ObjId) -> Option<ObjectDescriptor> {
        self.obj_arena.get(id)
    }

    /// Convenience helper to intern a scalar atom.
    pub fn intern_scalar(&self, atom: ScalarAtom) -> ScalarId {
        self.scalar_arena.intern(atom)
    }

    /// Convenience helper to retrieve a scalar atom.
    pub fn get_scalar(&self, id: ScalarId) -> Option<ScalarAtom> {
        self.scalar_arena.get(id)
    }

    /// Convenience helper to intern an open hypergraph diagram.
    pub fn intern_diagram(&self, diag: OpenHypergraph) -> DiagramId {
        self.hyper_arena.intern(diag)
    }

    /// Convenience helper to retrieve an open hypergraph diagram.
    pub fn get_diagram(&self, id: DiagramId) -> Option<OpenHypergraph> {
        self.hyper_arena.get(id)
    }

    /// Convenience helper to intern a symbolic dimension name.
    pub fn intern_sym_dim(&self, name: &str) -> SymDimId {
        self.sym_dim_arena.intern(name)
    }

    /// Convenience helper to retrieve a symbolic dimension name.
    pub fn get_sym_dim(&self, id: SymDimId) -> Option<String> {
        self.sym_dim_arena.get(id)
    }
}

thread_local! {
    /// Ambient thread-local context handle to allow operator overloading without lifetime annotations.
    static AMBIENT_CTX: RefCell<Option<Arc<CatRuntimeContext>>> = const { RefCell::new(None) };
}

/// Set the ambient thread-local context for the current thread.
pub fn set_ambient_context(ctx: Arc<CatRuntimeContext>) {
    AMBIENT_CTX.with(|cell| {
        *cell.borrow_mut() = Some(ctx);
    });
}

/// Execute a closure with the ambient thread-local context.
pub fn with_ambient_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&CatRuntimeContext) -> R,
{
    AMBIENT_CTX.with(|cell| {
        let guard = cell.borrow();
        guard.as_ref().map(|ctx| f(ctx))
    })
}
