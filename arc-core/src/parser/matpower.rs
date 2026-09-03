//! Deterministic tabular parser for MATPOWER `.m` benchmark case files.
//!
//! Parses the standard tabular matrices (`mpc.baseMVA`, `mpc.bus`, `mpc.gen`, `mpc.branch`)
//! without requiring a full MATLAB language grammar runtime.

use crate::model::{Branch, Bus, BusType, Generator, Load, ModelError, Network, Shunt};
use std::collections::BTreeMap;
use std::fmt;

/// Errors arising during MATPOWER case parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// A required matrix block (e.g. `mpc.bus`) was not found.
    MissingBlock(String),
    /// Failed to parse a numeric value in a matrix.
    InvalidNumber {
        /// Token text that failed to parse.
        token: String,
        /// Description of the context.
        context: String,
    },
    /// A matrix row had fewer columns than required.
    InsufficientColumns {
        /// Matrix block name.
        block: String,
        /// Row index (0-based).
        row: usize,
        /// Expected minimum columns.
        expected: usize,
        /// Found columns.
        found: usize,
    },
    /// Unsupported or invalid bus type code encountered.
    InvalidBusType(i32),
    /// Underlying model validation error.
    Model(ModelError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBlock(block) => write!(f, "Missing required MATPOWER block: {block}"),
            Self::InvalidNumber { token, context } => {
                write!(f, "Invalid number '{token}' in {context}")
            }
            Self::InsufficientColumns {
                block,
                row,
                expected,
                found,
            } => write!(
                f,
                "Insufficient columns in {block} row {row}: expected at least {expected}, found {found}"
            ),
            Self::InvalidBusType(t) => write!(f, "Invalid bus type code {t} (expected 1, 2, or 3)"),
            Self::Model(err) => write!(f, "Model validation error: {err}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<ModelError> for ParseError {
    fn from(err: ModelError) -> Self {
        Self::Model(err)
    }
}

/// Parser for standard MATPOWER `.m` case files.
pub struct MatpowerParser;

impl MatpowerParser {
    /// Parses a MATPOWER `.m` case file string into a validated `Network`.
    ///
    /// If `zero_index` is true, 1-based MATPOWER bus numbers are mapped to
    /// continuous 0-based indices ($0 \dots N-1$).
    pub fn parse(content: &str, zero_index: bool) -> Result<Network, ParseError> {
        let base_mva = Self::parse_base_mva(content).unwrap_or(100.0);
        let mut network = Network::new(base_mva);

        let bus_rows = Self::extract_matrix(content, "bus")?;
        let gen_rows = Self::extract_matrix(content, "gen")?;
        let branch_rows = Self::extract_matrix(content, "branch")?;

        // First pass on generators: collect voltage setpoints for PV buses
        let mut gen_vm_setpoints = BTreeMap::new();
        for (row_idx, row) in gen_rows.iter().enumerate() {
            if row.len() < 6 {
                return Err(ParseError::InsufficientColumns {
                    block: "mpc.gen".into(),
                    row: row_idx,
                    expected: 6,
                    found: row.len(),
                });
            }
            let raw_bus_id = row[0].round() as usize;
            let vg = row[5];
            gen_vm_setpoints.insert(raw_bus_id, vg);
        }

        // Map bus IDs: raw_bus_id -> mapped_id
        let mut bus_id_map = BTreeMap::new();
        for (idx, row) in bus_rows.iter().enumerate() {
            if row.is_empty() {
                continue;
            }
            let raw_id = row[0].round() as usize;
            let mapped_id = if zero_index { idx } else { raw_id };
            bus_id_map.insert(raw_id, mapped_id);
        }

        // Parse buses, loads, and shunts
        for (row_idx, row) in bus_rows.iter().enumerate() {
            if row.len() < 13 {
                return Err(ParseError::InsufficientColumns {
                    block: "mpc.bus".into(),
                    row: row_idx,
                    expected: 13,
                    found: row.len(),
                });
            }

            let raw_id = row[0].round() as usize;
            let bus_id = bus_id_map[&raw_id];
            let bus_type_code = row[1].round() as i32;
            let bus_type = match bus_type_code {
                1 => BusType::PQ,
                2 => BusType::PV,
                3 => BusType::Slack,
                4 => {
                    // Isolated bus, skip in standard power flow
                    continue;
                }
                other => return Err(ParseError::InvalidBusType(other)),
            };

            let pd = row[2];
            let qd = row[3];
            let gs = row[4];
            let bs = row[5];
            let vm_init = match bus_type {
                BusType::Slack | BusType::PV => *gen_vm_setpoints.get(&raw_id).unwrap_or(&row[7]),
                BusType::PQ => 1.0,
            };
            let va_deg = row[8];
            let base_kv = if row[9] > 0.0 { row[9] } else { 138.0 };
            let vmax = row[11];
            let vmin = row[12];

            let bus = Bus::new(bus_id, bus_type, base_kv)
                .with_name(format!("Bus {raw_id}"))
                .with_vm_pu(vm_init)
                .with_va_deg(va_deg)
                .with_voltage_limits(vmin, vmax);
            network.add_bus(bus)?;

            // Add load if non-zero
            if pd.abs() > 1e-6 || qd.abs() > 1e-6 {
                let load_id = network.loads.len();
                let load =
                    Load::new(load_id, bus_id, pd, qd).with_name(format!("Load at Bus {raw_id}"));
                network.add_load(load)?;
            }

            // Add bus shunt if non-zero
            if gs.abs() > 1e-6 || bs.abs() > 1e-6 {
                let shunt_id = network.shunts.len();
                let shunt = Shunt::new(shunt_id, bus_id, gs, bs)
                    .with_name(format!("Shunt at Bus {raw_id}"));
                network.add_shunt(shunt)?;
            }
        }

        // Parse generators
        for (row_idx, row) in gen_rows.iter().enumerate() {
            if row.len() < 10 {
                return Err(ParseError::InsufficientColumns {
                    block: "mpc.gen".into(),
                    row: row_idx,
                    expected: 10,
                    found: row.len(),
                });
            }

            let raw_bus = row[0].round() as usize;
            let bus_id = match bus_id_map.get(&raw_bus) {
                Some(&id) => id,
                None => continue, // If bus was isolated/skipped
            };

            let pg = row[1];
            let qg = row[2];
            let qmax = row[3];
            let qmin = row[4];
            let vg = row[5];
            let status = row[7].round() as i32 > 0;
            let pmax = row[8];
            let pmin = row[9];

            let gen = Generator::new(row_idx, bus_id, pg, vg)
                .with_name(format!("Gen {} at Bus {raw_bus}", row_idx + 1))
                .with_q_limits(qmin, qmax)
                .with_p_limits(pmin, pmax)
                .with_status(status);
            let mut gen = gen;
            gen.q_mvar = qg;
            network.add_generator(gen)?;
        }

        // Parse branches
        for (row_idx, row) in branch_rows.iter().enumerate() {
            if row.len() < 11 {
                return Err(ParseError::InsufficientColumns {
                    block: "mpc.branch".into(),
                    row: row_idx,
                    expected: 11,
                    found: row.len(),
                });
            }

            let raw_fbus = row[0].round() as usize;
            let raw_tbus = row[1].round() as usize;

            let fbus = match bus_id_map.get(&raw_fbus) {
                Some(&id) => id,
                None => continue,
            };
            let tbus = match bus_id_map.get(&raw_tbus) {
                Some(&id) => id,
                None => continue,
            };

            let r = row[2];
            let x = row[3];
            let b = row[4];
            let rate_a = if row[5] > 0.0 { Some(row[5]) } else { None };
            let ratio = if row[8] > 0.0 { row[8] } else { 1.0 };
            let angle_deg = row[9];
            let shift_rad = angle_deg.to_radians();
            let status = row[10].round() as i32 > 0;

            let is_transformer = (ratio - 1.0).abs() > 1e-5 || shift_rad.abs() > 1e-5;

            let branch = if is_transformer {
                Branch::new_transformer(row_idx, fbus, tbus, r, x, ratio, shift_rad)
                    .with_b_pu(b)
                    .with_name(format!("Trafo {raw_fbus}-{raw_tbus}"))
                    .with_status(status)
            } else {
                Branch::new_line(row_idx, fbus, tbus, r, x)
                    .with_b_pu(b)
                    .with_name(format!("Line {raw_fbus}-{raw_tbus}"))
                    .with_status(status)
            };

            let branch = if let Some(rating) = rate_a {
                branch.with_rating_mva(rating)
            } else {
                branch
            };

            network.add_branch(branch)?;
        }

        network.validate()?;
        Ok(network)
    }

    fn parse_base_mva(content: &str) -> Option<f64> {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('%') {
                continue;
            }
            if let Some(pos) = line.find("mpc.baseMVA") {
                let rest = &line[pos + "mpc.baseMVA".len()..];
                let eq_pos = rest.find('=')?;
                let val_str = rest[eq_pos + 1..].trim().trim_end_matches(';').trim();
                return val_str.parse::<f64>().ok();
            }
        }
        None
    }

    fn extract_matrix(content: &str, matrix_name: &str) -> Result<Vec<Vec<f64>>, ParseError> {
        let pattern = format!("mpc.{matrix_name}");
        let mut in_matrix = false;
        let mut rows = Vec::new();

        for line in content.lines() {
            let mut line = line.trim();

            // Strip comments
            if let Some(comment_pos) = line.find('%') {
                line = line[..comment_pos].trim();
            }

            if line.is_empty() {
                continue;
            }

            if !in_matrix {
                if let Some(pos) = line.find(&pattern) {
                    let after_pat = &line[pos + pattern.len()..];
                    if let Some(bracket_pos) = after_pat.find('[') {
                        in_matrix = true;
                        let after_bracket = after_pat[bracket_pos + 1..].trim();
                        if !after_bracket.is_empty() && !after_bracket.starts_with(']') {
                            Self::parse_matrix_line(after_bracket, matrix_name, &mut rows)?;
                        }
                    }
                }
            } else {
                if let Some(bracket_end) = line.find(']') {
                    let before_bracket = line[..bracket_end].trim();
                    if !before_bracket.is_empty() {
                        Self::parse_matrix_line(before_bracket, matrix_name, &mut rows)?;
                    }
                    in_matrix = false;
                    break;
                } else {
                    Self::parse_matrix_line(line, matrix_name, &mut rows)?;
                }
            }
        }

        if rows.is_empty() && in_matrix {
            return Err(ParseError::MissingBlock(format!("mpc.{matrix_name}")));
        }

        Ok(rows)
    }

    fn parse_matrix_line(
        line: &str,
        matrix_name: &str,
        rows: &mut Vec<Vec<f64>>,
    ) -> Result<(), ParseError> {
        // Line can contain multiple rows separated by semicolons
        let row_chunks: Vec<&str> = line.split(';').collect();

        for chunk in row_chunks {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }

            let mut current_row = Vec::new();
            for token in chunk.split_whitespace() {
                let token = token.trim().trim_end_matches(';');
                if token.is_empty() {
                    continue;
                }
                match token.parse::<f64>() {
                    Ok(val) => current_row.push(val),
                    Err(_) => {
                        // Support 'nan' or 'inf'
                        if token.eq_ignore_ascii_case("nan") {
                            current_row.push(f64::NAN);
                        } else if token.eq_ignore_ascii_case("inf") {
                            current_row.push(f64::INFINITY);
                        } else if token.eq_ignore_ascii_case("-inf") {
                            current_row.push(f64::NEG_INFINITY);
                        } else {
                            return Err(ParseError::InvalidNumber {
                                token: token.to_string(),
                                context: format!("mpc.{matrix_name}"),
                            });
                        }
                    }
                }
            }

            if !current_row.is_empty() {
                rows.push(current_row);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_case() {
        let content = r#"
            mpc.version = '2';
            mpc.baseMVA = 100.0;

            % Bus data
            mpc.bus = [
                1 3 0 0 0 0 1 1.0 0 138 1 1.1 0.9;
                2 1 50 20 0 0 1 1.0 0 138 1 1.1 0.9;
            ];

            % Gen data
            mpc.gen = [
                1 50 20 100 -100 1.0 100 1 100 0;
            ];

            % Branch data
            mpc.branch = [
                1 2 0.01 0.05 0.02 100 0 0 1.0 0 1 -360 360;
            ];
        "#;

        let net = MatpowerParser::parse(content, true).unwrap();
        assert_eq!(net.buses.len(), 2);
        assert_eq!(net.generators.len(), 1);
        assert_eq!(net.loads.len(), 1);
        assert_eq!(net.branches.len(), 1);
        assert_eq!(net.base_mva, 100.0);
    }
}
