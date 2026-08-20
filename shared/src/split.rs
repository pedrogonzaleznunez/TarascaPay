//! Reparto de gastos y simplificación de deudas.
//!
//! Toda la aritmética es entera (centavos). El reparto usa el método del
//! *resto mayor*: se asignan las partes enteras y los centavos sobrantes van a
//! quienes tienen el resto fraccionario más grande. Así la suma de las partes
//! siempre coincide exactamente con el total del gasto — nunca se pierde ni se
//! inventa un centavo.

use std::fmt;

use uuid::Uuid;

/// Estrategias de reparto soportadas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitType {
    /// Partes iguales entre todos los participantes.
    Equal,
    /// Cada participante aporta un importe exacto en centavos.
    Exact,
    /// Porcentajes en puntos básicos (10000 = 100 %).
    Percentage,
    /// Partes proporcionales (ej. 2 partes para uno, 1 para otro).
    Shares,
}

impl SplitType {
    pub fn as_str(self) -> &'static str {
        match self {
            SplitType::Equal => "equal",
            SplitType::Exact => "exact",
            SplitType::Percentage => "percentage",
            SplitType::Shares => "shares",
        }
    }

    pub fn parse(value: &str) -> Result<Self, SplitError> {
        match value {
            "equal" => Ok(SplitType::Equal),
            "exact" => Ok(SplitType::Exact),
            "percentage" => Ok(SplitType::Percentage),
            "shares" => Ok(SplitType::Shares),
            other => Err(SplitError::UnknownSplitType(other.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SplitError {
    UnknownSplitType(String),
    NoParticipants,
    NonPositiveAmount,
    NegativeValue,
    /// Los importes exactos no suman el total del gasto.
    ExactMismatch { expected: i64, got: i64 },
    /// Los porcentajes no suman 100 %.
    PercentageMismatch { got: i64 },
    /// Todas las partes son cero.
    ZeroShares,
    DuplicateParticipant,
}

impl fmt::Display for SplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitError::UnknownSplitType(v) => write!(f, "tipo de reparto desconocido: '{v}'"),
            SplitError::NoParticipants => write!(f, "el gasto necesita al menos un participante"),
            SplitError::NonPositiveAmount => write!(f, "el importe debe ser mayor a cero"),
            SplitError::NegativeValue => write!(f, "los valores del reparto no pueden ser negativos"),
            SplitError::ExactMismatch { expected, got } => write!(
                f,
                "los importes del reparto suman {got} y el gasto es de {expected}"
            ),
            SplitError::PercentageMismatch { got } => write!(
                f,
                "los porcentajes deben sumar 100 % (suman {:.2} %)",
                *got as f64 / 100.0
            ),
            SplitError::ZeroShares => write!(f, "al menos un participante debe tener partes"),
            SplitError::DuplicateParticipant => {
                write!(f, "hay participantes repetidos en el reparto")
            }
        }
    }
}

impl std::error::Error for SplitError {}

/// Entrada del reparto: participante y su valor crudo según [`SplitType`].
#[derive(Clone, Copy, Debug)]
pub struct Participant {
    pub user_id: Uuid,
    pub value: i64,
}

/// Resultado del reparto: cuánto le corresponde pagar a cada participante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Share {
    pub user_id: Uuid,
    pub share_cents: i64,
    pub split_value: i64,
}

/// Reparte `amount_cents` entre `participants`.
///
/// El resultado siempre cumple `sum(share_cents) == amount_cents`.
pub fn compute_shares(
    amount_cents: i64,
    split_type: SplitType,
    participants: &[Participant],
) -> Result<Vec<Share>, SplitError> {
    if amount_cents <= 0 {
        return Err(SplitError::NonPositiveAmount);
    }
    if participants.is_empty() {
        return Err(SplitError::NoParticipants);
    }
    if participants.iter().any(|p| p.value < 0) {
        return Err(SplitError::NegativeValue);
    }
    if has_duplicates(participants) {
        return Err(SplitError::DuplicateParticipant);
    }

    match split_type {
        SplitType::Equal => {
            let weights: Vec<i64> = participants.iter().map(|_| 1).collect();
            let shares = distribute(amount_cents, &weights);
            Ok(zip_shares(participants, &shares, |_| 0))
        }
        SplitType::Exact => {
            let total: i64 = participants.iter().map(|p| p.value).sum();
            if total != amount_cents {
                return Err(SplitError::ExactMismatch {
                    expected: amount_cents,
                    got: total,
                });
            }
            Ok(participants
                .iter()
                .map(|p| Share {
                    user_id: p.user_id,
                    share_cents: p.value,
                    split_value: p.value,
                })
                .collect())
        }
        SplitType::Percentage => {
            let total: i64 = participants.iter().map(|p| p.value).sum();
            if total != 10_000 {
                return Err(SplitError::PercentageMismatch { got: total });
            }
            let weights: Vec<i64> = participants.iter().map(|p| p.value).collect();
            let shares = distribute(amount_cents, &weights);
            Ok(zip_shares(participants, &shares, |p| p.value))
        }
        SplitType::Shares => {
            let total: i64 = participants.iter().map(|p| p.value).sum();
            if total <= 0 {
                return Err(SplitError::ZeroShares);
            }
            let weights: Vec<i64> = participants.iter().map(|p| p.value).collect();
            let shares = distribute(amount_cents, &weights);
            Ok(zip_shares(participants, &shares, |p| p.value))
        }
    }
}

fn has_duplicates(participants: &[Participant]) -> bool {
    for (i, a) in participants.iter().enumerate() {
        if participants[i + 1..].iter().any(|b| b.user_id == a.user_id) {
            return true;
        }
    }
    false
}

fn zip_shares(
    participants: &[Participant],
    shares: &[i64],
    value_of: impl Fn(&Participant) -> i64,
) -> Vec<Share> {
    participants
        .iter()
        .zip(shares)
        .map(|(p, &share_cents)| Share {
            user_id: p.user_id,
            share_cents,
            split_value: value_of(p),
        })
        .collect()
}

/// Reparte `total` proporcionalmente a `weights` usando el método del resto mayor.
///
/// Garantiza que la suma del resultado sea exactamente `total`.
fn distribute(total: i64, weights: &[i64]) -> Vec<i64> {
    let weight_sum: i128 = weights.iter().map(|&w| w as i128).sum();
    if weight_sum == 0 {
        return vec![0; weights.len()];
    }

    let total_128 = total as i128;
    let mut result: Vec<i64> = Vec::with_capacity(weights.len());
    // (resto, índice) para desempatar de forma determinística.
    let mut remainders: Vec<(i128, usize)> = Vec::with_capacity(weights.len());

    for (idx, &w) in weights.iter().enumerate() {
        let numerator = total_128 * w as i128;
        result.push((numerator / weight_sum) as i64);
        remainders.push((numerator % weight_sum, idx));
    }

    let assigned: i128 = result.iter().map(|&r| r as i128).sum();
    let mut leftover = total_128 - assigned;

    // Resto más grande primero; ante empate, el índice menor.
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    let mut cursor = 0usize;
    while leftover > 0 && !remainders.is_empty() {
        result[remainders[cursor % remainders.len()].1] += 1;
        leftover -= 1;
        cursor += 1;
    }

    result
}

// ---------------------------------------------------------------------------
// Simplificación de deudas
// ---------------------------------------------------------------------------

/// Saldo neto de una persona: positivo = le deben, negativo = debe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetBalance {
    pub user_id: Uuid,
    pub net_cents: i64,
}

/// Transferencia sugerida para saldar cuentas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub from: Uuid,
    pub to: Uuid,
    pub amount_cents: i64,
}

/// Calcula el conjunto mínimo (heurístico) de transferencias que salda todos
/// los saldos.
///
/// Estrategia voraz: se empareja repetidamente al mayor deudor con el mayor
/// acreedor. Para N personas produce como mucho N-1 transferencias, muy por
/// debajo de las N·(N-1)/2 que saldrían de pagar deuda por deuda.
pub fn simplify_debts(balances: &[NetBalance]) -> Vec<Transfer> {
    // Orden determinístico: por importe y, ante empate, por id.
    let mut debtors: Vec<(Uuid, i64)> = balances
        .iter()
        .filter(|b| b.net_cents < 0)
        .map(|b| (b.user_id, -b.net_cents))
        .collect();
    let mut creditors: Vec<(Uuid, i64)> = balances
        .iter()
        .filter(|b| b.net_cents > 0)
        .map(|b| (b.user_id, b.net_cents))
        .collect();

    debtors.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    creditors.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut transfers = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);

    while i < debtors.len() && j < creditors.len() {
        let amount = debtors[i].1.min(creditors[j].1);
        if amount > 0 {
            transfers.push(Transfer {
                from: debtors[i].0,
                to: creditors[j].0,
                amount_cents: amount,
            });
            debtors[i].1 -= amount;
            creditors[j].1 -= amount;
        }
        if debtors[i].1 == 0 {
            i += 1;
        }
        if creditors[j].1 == 0 {
            j += 1;
        }
    }

    transfers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<Uuid> {
        (0..n)
            .map(|i| Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0000u128 + i as u128))
            .collect()
    }

    fn parts(ids: &[Uuid], values: &[i64]) -> Vec<Participant> {
        ids.iter()
            .zip(values)
            .map(|(&user_id, &value)| Participant { user_id, value })
            .collect()
    }

    #[test]
    fn equal_split_divides_evenly() {
        let u = ids(4);
        let p = parts(&u, &[0, 0, 0, 0]);
        let shares = compute_shares(10_000, SplitType::Equal, &p).unwrap();
        assert!(shares.iter().all(|s| s.share_cents == 2_500));
    }

    #[test]
    fn equal_split_never_loses_a_cent() {
        let u = ids(3);
        let p = parts(&u, &[0, 0, 0]);
        // 100.00 entre 3 => 33.34 / 33.33 / 33.33
        let shares = compute_shares(10_000, SplitType::Equal, &p).unwrap();
        let total: i64 = shares.iter().map(|s| s.share_cents).sum();
        assert_eq!(total, 10_000);
        let mut cents: Vec<i64> = shares.iter().map(|s| s.share_cents).collect();
        cents.sort();
        assert_eq!(cents, vec![3_333, 3_333, 3_334]);
    }

    #[test]
    fn equal_split_of_one_cent_goes_to_a_single_person() {
        let u = ids(3);
        let p = parts(&u, &[0, 0, 0]);
        let shares = compute_shares(1, SplitType::Equal, &p).unwrap();
        let total: i64 = shares.iter().map(|s| s.share_cents).sum();
        assert_eq!(total, 1);
        assert_eq!(shares.iter().filter(|s| s.share_cents == 1).count(), 1);
    }

    #[test]
    fn exact_split_requires_matching_total() {
        let u = ids(2);
        let ok = compute_shares(5_000, SplitType::Exact, &parts(&u, &[2_000, 3_000]));
        assert!(ok.is_ok());

        let bad = compute_shares(5_000, SplitType::Exact, &parts(&u, &[2_000, 2_000]));
        assert_eq!(
            bad.unwrap_err(),
            SplitError::ExactMismatch {
                expected: 5_000,
                got: 4_000
            }
        );
    }

    #[test]
    fn percentage_split_requires_one_hundred_percent() {
        let u = ids(2);
        // 70 % / 30 % sobre 100.00
        let shares =
            compute_shares(10_000, SplitType::Percentage, &parts(&u, &[7_000, 3_000])).unwrap();
        assert_eq!(shares[0].share_cents, 7_000);
        assert_eq!(shares[1].share_cents, 3_000);

        let bad = compute_shares(10_000, SplitType::Percentage, &parts(&u, &[7_000, 2_000]));
        assert_eq!(
            bad.unwrap_err(),
            SplitError::PercentageMismatch { got: 9_000 }
        );
    }

    #[test]
    fn percentage_split_with_thirds_stays_exact() {
        let u = ids(3);
        let shares = compute_shares(
            10_000,
            SplitType::Percentage,
            &parts(&u, &[3_333, 3_333, 3_334]),
        )
        .unwrap();
        let total: i64 = shares.iter().map(|s| s.share_cents).sum();
        assert_eq!(total, 10_000);
    }

    #[test]
    fn shares_split_is_proportional() {
        let u = ids(3);
        // Una pareja (2 partes) y una persona sola (1 parte) sobre 90.00
        let shares = compute_shares(9_000, SplitType::Shares, &parts(&u, &[2, 1, 0])).unwrap();
        assert_eq!(shares[0].share_cents, 6_000);
        assert_eq!(shares[1].share_cents, 3_000);
        assert_eq!(shares[2].share_cents, 0);
    }

    #[test]
    fn shares_split_rejects_all_zero() {
        let u = ids(2);
        let bad = compute_shares(1_000, SplitType::Shares, &parts(&u, &[0, 0]));
        assert_eq!(bad.unwrap_err(), SplitError::ZeroShares);
    }

    #[test]
    fn rejects_bad_inputs() {
        let u = ids(2);
        assert_eq!(
            compute_shares(0, SplitType::Equal, &parts(&u, &[0, 0])).unwrap_err(),
            SplitError::NonPositiveAmount
        );
        assert_eq!(
            compute_shares(100, SplitType::Equal, &[]).unwrap_err(),
            SplitError::NoParticipants
        );
        assert_eq!(
            compute_shares(100, SplitType::Shares, &parts(&u, &[-1, 2])).unwrap_err(),
            SplitError::NegativeValue
        );
        let dup = vec![
            Participant {
                user_id: u[0],
                value: 1
            },
            Participant {
                user_id: u[0],
                value: 1
            },
        ];
        assert_eq!(
            compute_shares(100, SplitType::Equal, &dup).unwrap_err(),
            SplitError::DuplicateParticipant
        );
    }

    #[test]
    fn simplify_settles_a_simple_triangle() {
        let u = ids(3);
        // A puso 60 de más, B debe 40, C debe 20.
        let balances = vec![
            NetBalance {
                user_id: u[0],
                net_cents: 6_000,
            },
            NetBalance {
                user_id: u[1],
                net_cents: -4_000,
            },
            NetBalance {
                user_id: u[2],
                net_cents: -2_000,
            },
        ];
        let transfers = simplify_debts(&balances);
        assert_eq!(transfers.len(), 2);
        assert!(transfers.iter().all(|t| t.to == u[0]));
        let total: i64 = transfers.iter().map(|t| t.amount_cents).sum();
        assert_eq!(total, 6_000);
    }

    #[test]
    fn simplify_produces_at_most_n_minus_one_transfers() {
        let u = ids(5);
        let balances = vec![
            NetBalance {
                user_id: u[0],
                net_cents: 10_000,
            },
            NetBalance {
                user_id: u[1],
                net_cents: 5_000,
            },
            NetBalance {
                user_id: u[2],
                net_cents: -3_000,
            },
            NetBalance {
                user_id: u[3],
                net_cents: -6_000,
            },
            NetBalance {
                user_id: u[4],
                net_cents: -6_000,
            },
        ];
        let transfers = simplify_debts(&balances);
        assert!(transfers.len() <= 4, "demasiadas transferencias");

        // Aplicar las transferencias debe dejar todos los saldos en cero.
        let mut net: Vec<(Uuid, i64)> = balances.iter().map(|b| (b.user_id, b.net_cents)).collect();
        for t in &transfers {
            net.iter_mut().find(|(id, _)| *id == t.from).unwrap().1 += t.amount_cents;
            net.iter_mut().find(|(id, _)| *id == t.to).unwrap().1 -= t.amount_cents;
        }
        assert!(net.iter().all(|(_, v)| *v == 0));
    }

    #[test]
    fn simplify_ignores_settled_groups() {
        let u = ids(2);
        let balances = vec![
            NetBalance {
                user_id: u[0],
                net_cents: 0,
            },
            NetBalance {
                user_id: u[1],
                net_cents: 0,
            },
        ];
        assert!(simplify_debts(&balances).is_empty());
    }
}
