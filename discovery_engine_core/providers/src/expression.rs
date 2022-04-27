// Copyright 2022 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Expressions

use std::collections::BTreeSet;

use itertools::{intersperse, Either, Itertools};
use maplit::btreeset;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Expr {
    // contained `Expr` must not be an `And`
    And(BTreeSet<Expr>),
    // contained `Expr` must not be an `Or`
    Or(BTreeSet<Expr>),
    Value(String),
}

impl Expr {
    #[allow(dead_code)]
    pub(crate) fn and(self, expr: impl Into<Expr>) -> Expr {
        let expr = expr.into();
        match (self, expr) {
            (Expr::And(mut vs_a), Expr::And(mut vs_b)) => {
                vs_a.append(&mut vs_b);
                if vs_a.len() == 1 {
                    vs_a.into_iter().next().unwrap(/* there is one */)
                } else {
                    Expr::And(vs_a)
                }
            }
            (Expr::And(mut exprs), expr) | (expr, Expr::And(mut exprs)) => {
                if exprs.is_empty() {
                    expr
                } else {
                    exprs.insert(expr);
                    Expr::And(exprs)
                }
            }
            (this, expr) => Expr::And(btreeset![this, expr]),
        }
    }

    pub(crate) fn or(self, expr: impl Into<Expr>) -> Expr {
        let expr = expr.into();
        match (self, expr) {
            (Expr::Or(mut vs_a), Expr::Or(mut vs_b)) => {
                vs_a.append(&mut vs_b);
                if vs_a.len() == 1 {
                    vs_a.into_iter().next().unwrap(/* there is one */)
                } else {
                    Expr::Or(vs_a)
                }
            }
            (Expr::Or(mut exprs), expr) | (expr, Expr::Or(mut exprs)) => {
                if exprs.is_empty() {
                    expr
                } else {
                    exprs.insert(expr);
                    Expr::Or(exprs)
                }
            }
            (this, expr) => Expr::Or(btreeset![this, expr]),
        }
    }

    /// Crate a new expression where all items in the iterator are in "or" with each other.
    pub(crate) fn or_from_iter(exprs: impl Iterator<Item = impl Into<Expr>>) -> Expr {
        let (ors, exprs): (BTreeSet<_>, BTreeSet<_>) = exprs.partition_map(|expr| {
            let expr = expr.into();

            match expr {
                Expr::Or(exprs) => Either::Left(exprs),
                expr => Either::Right(expr),
            }
        });

        // reuse some logic
        Expr::Or(btreeset![]).or(Expr::Or(
            ors.into_iter().flatten().chain(exprs.into_iter()).collect(),
        ))
    }

    pub(crate) fn build(self) -> String {
        match self {
            Expr::And(exprs) => intersperse_exprs(exprs.into_iter(), " AND "),
            Expr::Or(exprs) => intersperse_exprs(exprs.into_iter(), " OR "),
            Expr::Value(s) => s,
        }
    }
}

fn intersperse_exprs(exprs: impl Iterator<Item = Expr>, sep: &str) -> String {
    intersperse(
        exprs.filter_map(|expr| {
            let need_parens = !matches!(expr, Expr::Value(_));

            let s = Some(expr.build()).filter(|s| !s.is_empty());
            if need_parens {
                s.map(|s| format!("({})", s))
            } else {
                s
            }
        }),
        sep.to_string(),
    )
    .collect()
}

impl From<String> for Expr {
    fn from(s: String) -> Self {
        Expr::Value(s)
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use super::*;

    #[test]
    fn test_and_simple() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());

        assert_eq!(Expr::And(btreeset![a.clone(), b.clone()]), a.and(b));
    }

    #[test]
    fn test_and_and() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());
        let expected = Expr::And(btreeset![a.clone(), b.clone(), c.clone()]);

        // a /\ (b /\ c) => a /\ b /\ c
        assert_eq!(expected, a.clone().and(b.clone().and(c.clone())));

        // (a /\ b) /\ c => a /\ b /\ c
        assert_eq!(expected, a.and(b).and(c));
    }

    #[test]
    fn test_and_and_empty() {
        let empty = Expr::And(btreeset![]);
        let a = Expr::Value("a".to_string());

        assert_eq!(a, empty.clone().and(a.clone()));

        let one = Expr::And(btreeset![a.clone()]);
        assert_eq!(a, empty.and(one));
    }

    #[test]
    fn test_and_or() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());
        let expected = Expr::And(btreeset![
            a.clone(),
            Expr::Or(btreeset![b.clone(), c.clone()])
        ]);

        // a /\ (b \/ c)
        assert_eq!(expected, a.clone().and(b.clone().or(c.clone())));

        // (b \/ c) /\ a
        assert_eq!(expected, b.or(c).and(a));
    }

    #[test]
    fn test_or_from_iter_empty() {
        assert_eq!(
            Expr::Or(btreeset![]),
            Expr::or_from_iter(iter::empty::<Expr>())
        );
    }

    #[test]
    fn test_or_from_iter_empty_one() {
        let a = Expr::Value("a".to_string());

        assert_eq!(a.clone(), Expr::or_from_iter(iter::once(a)));
    }

    #[test]
    fn test_or_from_iter_more() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());

        assert_eq!(
            Expr::Or(btreeset![a.clone(), b.clone(), c.clone()]),
            Expr::or_from_iter(btreeset![a, b, c].into_iter())
        );
    }

    #[test]
    fn test_or_from_iter_flat() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());
        let expected = Expr::Or(btreeset![a.clone(), b.clone(), c.clone()]);

        assert_eq!(
            expected,
            Expr::or_from_iter(btreeset![a.clone(), b.clone().or(c.clone())].into_iter())
        );

        assert_eq!(
            expected,
            Expr::or_from_iter(btreeset![b.clone().or(c.clone()), a.clone()].into_iter())
        );

        assert_eq!(
            expected,
            Expr::or_from_iter(btreeset![b.or(c).or(a)].into_iter())
        );
    }

    #[test]
    fn test_or_simple() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());

        assert_eq!(Expr::Or(btreeset![a.clone(), b.clone()]), a.or(b));
    }

    #[test]
    fn test_or_or() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());
        let expected = Expr::Or(btreeset![a.clone(), b.clone(), c.clone()]);

        // a \/ (b \/ c) => a \/ b \/ c
        assert_eq!(expected, a.clone().or(b.clone().or(c.clone())));

        // (a \/ b) \/ c => a \/ b \/ c
        assert_eq!(expected, a.or(b).or(c));
    }

    #[test]
    fn test_or_or_empty() {
        let empty = Expr::Or(btreeset![]);
        let a = Expr::Value("a".to_string());

        assert_eq!(a, empty.clone().or(a.clone()));

        let one = Expr::Or(btreeset![a.clone()]);
        assert_eq!(a, empty.or(one));
    }

    #[test]
    fn test_or_and() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());
        let expected = Expr::Or(btreeset![
            a.clone(),
            Expr::And(btreeset![b.clone(), c.clone()])
        ]);

        // a \/ (b /\ c)
        assert_eq!(expected, a.clone().or(b.clone().and(c.clone())));

        // (b /\ c) \/ a
        assert_eq!(expected, b.and(c).or(a));
    }

    #[test]
    fn test_build_value_empty() {
        let empty = "".to_string();

        assert_eq!(empty, Expr::Value(empty.clone()).build());
    }

    #[test]
    fn test_build_value() {
        let a = "a".to_string();

        assert_eq!(a, Expr::Value(a.clone()).build());
    }

    #[test]
    fn test_build_and_empty() {
        let v = Expr::Value("".to_string());

        let empty_and = v.clone().and(v.clone());
        assert_eq!("", empty_and.build());

        let empty_and_empty_and = v.clone().or(v.clone().or(v.clone()));
        assert_eq!("", empty_and_empty_and.build());

        let empty_and_empty_or = v.clone().and(v.clone().or(v));
        assert_eq!("", empty_and_empty_or.build());
    }

    #[test]
    fn test_build_and_values() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());

        assert_eq!(
            "a AND b AND c",
            a.clone().and(b.clone()).and(c.clone()).build()
        );
        assert_eq!("a AND b AND c", c.and(b.and(a)).build());
    }

    #[test]
    fn test_build_or_empty() {
        let v = Expr::Value("".to_string());

        let empty_or = v.clone().or(v.clone());
        assert_eq!("", empty_or.build());

        let empty_or_empty_or = v.clone().or(v.clone().or(v.clone()));
        assert_eq!("", empty_or_empty_or.build());

        let empty_or_empty_and = v.clone().or(v.clone().and(v));
        assert_eq!("", empty_or_empty_and.build());
    }

    #[test]
    fn test_build_or_values() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());

        assert_eq!("a OR b OR c", a.clone().or(b.clone()).or(c.clone()).build());
        assert_eq!("a OR b OR c", c.or(b.or(a)).build());
    }

    #[test]
    fn test_build_and_or() {
        let a = Expr::Value("a".to_string());
        let b = Expr::Value("b".to_string());
        let c = Expr::Value("c".to_string());
        let d = Expr::Value("d".to_string());

        // (a /\ b) \/ c
        assert_eq!(
            "(a AND b) OR c",
            a.clone().and(b.clone()).or(c.clone()).build()
        );

        // a /\ (b \/ c)
        assert_eq!(
            "(b OR c) AND a",
            a.clone().and(b.clone().or(c.clone())).build()
        );

        // (a \/ b) /\ c) \/ d
        assert_eq!(
            "((a OR b) AND c) OR d",
            a.clone().or(b.clone()).and(c.clone()).or(d.clone()).build()
        );

        // (a /\ b) \/ c) /\ d
        assert_eq!("((a AND b) OR c) AND d", a.and(b).or(c).and(d).build());
    }
}
