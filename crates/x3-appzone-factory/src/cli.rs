//! CLI harness: command parsing and dispatch for the `x3-appzone` binary.
//!
//! This module is intentionally thin.  Real I/O (reading files, submitting
//! extrinsics via RPC) is injected through trait objects so the business
//! logic remains testable without a live node.

use alloc::string::String;
use alloc::vec::Vec;

use crate::templates::Param;

/// Parsed CLI command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// List available templates.
    ListTemplates,
    /// Deploy a new AppZone.
    Deploy {
        template_id_hex: String,
        zone_name: String,
        params: Vec<Param>,
    },
    /// Show the status of an existing zone.
    Status { zone_id_hex: String },
    /// Pause a running zone.
    Pause { zone_id_hex: String },
    /// Activate a paused zone.
    Activate { zone_id_hex: String },
    /// Decommission a zone permanently.
    Decommission { zone_id_hex: String },
}

/// Parse a minimal argv-style slice into a `Cmd`.
///
/// Format:
/// ```text
/// list-templates
/// deploy <template_id_hex> <zone_name> [key=value ...]
/// status <zone_id_hex>
/// pause <zone_id_hex>
/// activate <zone_id_hex>
/// decommission <zone_id_hex>
/// ```
pub fn parse_args(args: &[&str]) -> Result<Cmd, CliError> {
    match args.first() {
        Some(&"list-templates") => Ok(Cmd::ListTemplates),
        Some(&"deploy") => {
            if args.len() < 3 {
                return Err(CliError::MissingArgument(
                    "deploy <template_id_hex> <zone_name>",
                ));
            }
            let params = args[3..]
                .iter()
                .map(|kv| parse_kv(kv))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Cmd::Deploy {
                template_id_hex: args[1].into(),
                zone_name: args[2].into(),
                params,
            })
        }
        Some(&"status") => require_id(args, "status").map(|id| Cmd::Status { zone_id_hex: id }),
        Some(&"pause") => require_id(args, "pause").map(|id| Cmd::Pause { zone_id_hex: id }),
        Some(&"activate") => {
            require_id(args, "activate").map(|id| Cmd::Activate { zone_id_hex: id })
        }
        Some(&"decommission") => {
            require_id(args, "decommission").map(|id| Cmd::Decommission { zone_id_hex: id })
        }
        Some(other) => Err(CliError::UnknownCommand((*other).into())),
        None => Err(CliError::NoCommand),
    }
}

fn require_id(args: &[&str], cmd: &'static str) -> Result<String, CliError> {
    args.get(1)
        .map(|s| (*s).into())
        .ok_or(CliError::MissingArgument(cmd))
}

fn parse_kv(kv: &str) -> Result<Param, CliError> {
    let mut parts = kv.splitn(2, '=');
    let key = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(CliError::InvalidParam(kv.into()))?;
    let value = parts.next().unwrap_or("");
    Ok(Param {
        key: key.into(),
        value: value.into(),
    })
}

/// CLI-layer error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliError {
    NoCommand,
    UnknownCommand(String),
    MissingArgument(&'static str),
    InvalidParam(String),
}
