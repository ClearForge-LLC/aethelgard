//! The specification schema, the solved graph, validation, and serialization
//! (`docs/architecture.md` §3). Every shared specification type lives here and
//! nowhere else (`INV-01`).
//!
//! Every type refuses unknown fields and accepts only an object, never serde's
//! positional-sequence form, and [`TopologyGraph::validate`] refuses rather
//! than defaults (`INV-06`).

#![forbid(unsafe_code)]

use std::collections::HashSet;

use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The top-level specification: one project, its voltage domains, and its net
/// constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct TopologyGraph {
    pub project_name: String,
    pub voltage_domains: Vec<VoltageDomain>,
    pub net_constraints: Vec<NetConstraint>,
}

/// A supply rail and how far it may stray from nominal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct VoltageDomain {
    pub id: String,
    pub nominal_millivolts: u32,
    pub tolerance_permille: u16,
}

/// The electrical bounds one net must stay within.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct NetConstraint {
    pub net_name: String,
    pub max_capacitance_femtofarads: u64,
    pub max_rise_time_nanoseconds: u32,
    /// Required as a key: `null` means unconstrained, and a missing key is a
    /// refusal rather than a silent `None` (`INV-06`).
    #[serde(deserialize_with = "Option::deserialize")]
    pub impedance_ohms: Option<u32>,
}

/// The derives above are `remote = "Self"`, so they generate inherent
/// functions; these trait impls wrap them so a type deserializes only from an
/// object. The derived form would also accept a positional sequence such as
/// `["p",[],[]]`, which bypasses field names entirely.
macro_rules! object_only {
    ($($ty:ident),*) => {$(
        impl Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                $ty::serialize(self, serializer)
            }
        }

        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct ObjectVisitor;

                impl<'de> Visitor<'de> for ObjectVisitor {
                    type Value = $ty;

                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str(concat!("a ", stringify!($ty), " object"))
                    }

                    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<$ty, A::Error> {
                        $ty::deserialize(MapAccessDeserializer::new(map))
                    }
                }

                deserializer.deserialize_map(ObjectVisitor)
            }
        }
    )*};
}

object_only!(TopologyGraph, VoltageDomain, NetConstraint);

/// One variant per refusal. Each names the offending id, or its index when the
/// id itself is what is missing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IrValidationError {
    #[error("project_name is empty or whitespace-only")]
    EmptyProjectName,
    #[error("voltage_domains[{index}] has an empty or whitespace-only id")]
    EmptyDomainId { index: usize },
    #[error("voltage domain id `{id}` appears more than once")]
    DuplicateDomainId { id: String },
    #[error("voltage domain `{id}` has nominal_millivolts 0")]
    ZeroNominalVoltage { id: String },
    #[error("voltage domain `{id}` has tolerance_permille {value}, outside 1..=500")]
    ToleranceOutOfRange { id: String, value: u16 },
    #[error("net_constraints[{index}] has an empty or whitespace-only net_name")]
    EmptyNetName { index: usize },
    #[error("net_name `{id}` appears more than once")]
    DuplicateNetName { id: String },
    #[error("net `{id}` has max_capacitance_femtofarads 0")]
    ZeroCapacitance { id: String },
    #[error("net `{id}` has max_rise_time_nanoseconds 0")]
    ZeroRiseTime { id: String },
    #[error("net `{id}` has impedance_ohms 0")]
    ZeroImpedance { id: String },
}

impl IrValidationError {
    /// The variant name, as callers see it in the CLI's error envelope.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::EmptyProjectName => "EmptyProjectName",
            Self::EmptyDomainId { .. } => "EmptyDomainId",
            Self::DuplicateDomainId { .. } => "DuplicateDomainId",
            Self::ZeroNominalVoltage { .. } => "ZeroNominalVoltage",
            Self::ToleranceOutOfRange { .. } => "ToleranceOutOfRange",
            Self::EmptyNetName { .. } => "EmptyNetName",
            Self::DuplicateNetName { .. } => "DuplicateNetName",
            Self::ZeroCapacitance { .. } => "ZeroCapacitance",
            Self::ZeroRiseTime { .. } => "ZeroRiseTime",
            Self::ZeroImpedance { .. } => "ZeroImpedance",
        }
    }

    /// The trimmed offending id, or `None` when the id is the thing missing.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::EmptyProjectName | Self::EmptyDomainId { .. } | Self::EmptyNetName { .. } => None,
            Self::DuplicateDomainId { id }
            | Self::ZeroNominalVoltage { id }
            | Self::ToleranceOutOfRange { id, .. }
            | Self::DuplicateNetName { id }
            | Self::ZeroCapacitance { id }
            | Self::ZeroRiseTime { id }
            | Self::ZeroImpedance { id } => Some(id),
        }
    }
}

impl TopologyGraph {
    /// Refuses the first malformed value found, in document order. Ids are
    /// compared trimmed, so ids differing only by surrounding whitespace are
    /// duplicates.
    pub fn validate(&self) -> Result<(), IrValidationError> {
        if self.project_name.trim().is_empty() {
            return Err(IrValidationError::EmptyProjectName);
        }

        let mut domain_ids = HashSet::new();
        for (index, domain) in self.voltage_domains.iter().enumerate() {
            let id = domain.id.trim();
            if id.is_empty() {
                return Err(IrValidationError::EmptyDomainId { index });
            }
            if !domain_ids.insert(id) {
                return Err(IrValidationError::DuplicateDomainId { id: id.to_owned() });
            }
            if domain.nominal_millivolts == 0 {
                return Err(IrValidationError::ZeroNominalVoltage { id: id.to_owned() });
            }
            if !(1..=500).contains(&domain.tolerance_permille) {
                return Err(IrValidationError::ToleranceOutOfRange {
                    id: id.to_owned(),
                    value: domain.tolerance_permille,
                });
            }
        }

        let mut net_names = HashSet::new();
        for (index, net) in self.net_constraints.iter().enumerate() {
            let id = net.net_name.trim();
            if id.is_empty() {
                return Err(IrValidationError::EmptyNetName { index });
            }
            if !net_names.insert(id) {
                return Err(IrValidationError::DuplicateNetName { id: id.to_owned() });
            }
            if net.max_capacitance_femtofarads == 0 {
                return Err(IrValidationError::ZeroCapacitance { id: id.to_owned() });
            }
            if net.max_rise_time_nanoseconds == 0 {
                return Err(IrValidationError::ZeroRiseTime { id: id.to_owned() });
            }
            if net.impedance_ohms == Some(0) {
                return Err(IrValidationError::ZeroImpedance { id: id.to_owned() });
            }
        }

        Ok(())
    }
}
