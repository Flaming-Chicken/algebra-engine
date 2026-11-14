//! Legacy bridge context and symbol/dummy-index translation tables.

use ahash::AHashMap;
use cat_core::{DimVector, ObjId, WireId};
use serde::{Deserialize, Serialize};

/// Mode controlling how legacy assumptions (e.g. unverified positive real domain)
/// are handled during migration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LegacyCompatibilityDomain {
    /// Permissive mode for legacy tests: attaches AssumedPositiveArchimedean predicate
    /// while recording a diagnostic log for auditing mathematically fragile steps.
    #[default]
    AssumedPositiveArchimedean,
    /// Strict mode: enforces rigorous non-zero divisor, branch-cut, and domain validity.
    StrictCategorical,
}

/// Bidirectional symbol & dummy-index translation table bridging the legacy library's
/// identifiers and named tensor indices to `cat-core` v2.1 arena handles.
#[derive(Default)]
pub struct LegacyBridgeContext {
    /// Maps legacy variable/function names or IDs to `(SymbolId, DefaultObjId, DimVector)`
    pub symbol_bijection: AHashMap<String, (u32, ObjId, DimVector)>,
    /// Reverse lookup for lowering `CatTerm` back to legacy AST identifiers
    pub reverse_symbols: AHashMap<u32, String>,
    /// Tracks open vs. contracted named indices during a single AST-to-Hypergraph lift pass.
    /// If a named index appears once -> added to `OpenHypergraph::boundary_ports`.
    /// If a named index appears twice (Einstein convention) -> fused into an internal `WireId`!
    pub active_index_wires: AHashMap<String, (WireId, bool)>,
    /// Reverse mapping from WireId to synthesized canonical dummy index name.
    pub wire_to_symbol: AHashMap<WireId, String>,
    /// Counter for generating new unique symbol handles.
    pub next_symbol_id: u32,
    /// Compatibility mode for handling legacy algebraic assumptions.
    pub compatibility: LegacyCompatibilityDomain,
    /// Telemetry log of assumptions made during translation.
    pub telemetry_audit: Vec<String>,
}

impl LegacyBridgeContext {
    /// Create a new legacy bridge context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a legacy named symbol.
    pub fn register_symbol(&mut self, name: &str, obj: ObjId, dim: DimVector) -> u32 {
        if let Some((id, _, _)) = self.symbol_bijection.get(name) {
            return *id;
        }
        let id = self.next_symbol_id;
        self.next_symbol_id += 1;
        self.symbol_bijection.insert(name.to_string(), (id, obj, dim));
        self.reverse_symbols.insert(id, name.to_string());
        id
    }

    /// Retrieve or register a symbol with standard dimensionless scalar sort.
    pub fn get_or_register_symbol(&mut self, name: &str) -> u32 {
        if let Some((id, _, _)) = self.symbol_bijection.get(name) {
            *id
        } else {
            self.register_symbol(name, ObjId(0), DimVector::DIMENSIONLESS)
        }
    }

    /// Reverse lookup: get symbol name from symbol ID.
    pub fn get_symbol_name(&self, id: u32) -> Option<&str> {
        self.reverse_symbols.get(&id).map(|s| s.as_str())
    }

    /// Log an implicit domain assumption for telemetry auditing.
    pub fn audit_assumption(&mut self, note: String) {
        tracing::warn!(target: "cat_bridge::retrofit", assumption = %note, "Legacy assumption engaged");
        self.telemetry_audit.push(note);
    }
}
