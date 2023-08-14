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

//! Module containing non-database specific sqlx utilities.

use std::iter;

use displaydoc::Display;
use sqlx::{Database, Encode, QueryBuilder, Type};

use crate::{error::common::InternalError, Error};

pub(super) trait SqlxPushTupleExt<'args, DB: Database> {
    fn push_tuple<T>(&mut self, as_tuple: T) -> &mut Self
    where
        T: SqlxPushAsTuple<'args, DB>;

    fn push_nested_tuple<I>(&mut self, values: IterAsTuple<I>) -> &mut Self
    where
        I: Iterator,
        I::Item: SqlxPushAsTuple<'args, DB>;
}

impl<'args, DB> SqlxPushTupleExt<'args, DB> for QueryBuilder<'args, DB>
where
    DB: Database,
{
    fn push_tuple<T>(&mut self, as_tuple: T) -> &mut Self
    where
        T: SqlxPushAsTuple<'args, DB>,
    {
        self.push("(");
        as_tuple.push_as_inner_tuple(self);
        self.push(")");
        self
    }

    fn push_nested_tuple<I>(&mut self, values: IterAsTuple<I>) -> &mut Self
    where
        I: Iterator,
        I::Item: SqlxPushAsTuple<'args, DB>,
    {
        self.push("(");
        self.push_tuple(values.first);
        for tuple in values.iter {
            self.push(", ");
            self.push_tuple(tuple);
        }
        self.push(")");
        self
    }
}

pub(super) trait SqlxPushAsTuple<'args, DB: Database> {
    fn push_as_inner_tuple(self, builder: &mut QueryBuilder<'args, DB>);
}

macro_rules! impl_sqlx_push_as_tuple {
    () => ();
    ($head:ident $(, $tail:ident)* $(,)?) => (
        impl<'args, DB, $head, $($tail),*> SqlxPushAsTuple<'args, DB> for ($head, $($tail),*)
        where
            DB: Database,
            $head: 'args + Encode<'args, DB> + Send + Type<DB>,
            $($tail: 'args + Encode<'args, DB> + Send + Type<DB>),*
        {
            fn push_as_inner_tuple(self, builder: &mut QueryBuilder<'args, DB>) {
                let mut separated = builder.separated(", ");
                #[allow(non_snake_case)]
                let ($head, $($tail),*) = self;
                separated.push_bind($head);
                $(separated.push_bind($tail);)*
            }
        }
        impl_sqlx_push_as_tuple! { $($tail),* }
    );
}

impl_sqlx_push_as_tuple! {
    T0, T1, T2, T3, T4, T5, T6, T7,
    T8, T9, T10, T11, T12, T13, T14, T15,
    T16, T17, T18, T19, T20, T21, T22, T23,
    T24, T25, T26, T27, T28, T29, T30, T31,
}

/// Empty tuples are not supported by SQL.
#[derive(Debug, Display, thiserror::Error)]
pub(super) struct UnsupportedEmptyTuple;

impl From<UnsupportedEmptyTuple> for Error {
    fn from(error: UnsupportedEmptyTuple) -> Self {
        InternalError::from_std(error).into()
    }
}

pub(super) struct IterAsTuple<I>
where
    I: Iterator,
{
    first: I::Item,
    iter: I,
}

impl<I> IterAsTuple<I>
where
    I: Iterator,
{
    pub(super) fn new<II>(iter: II) -> Result<Self, UnsupportedEmptyTuple>
    where
        II: IntoIterator<IntoIter = I>,
    {
        let mut iter = iter.into_iter();
        let Some(first) = iter.next() else {
            return Err(UnsupportedEmptyTuple);
        };
        Ok(Self { first, iter })
    }

    pub(super) fn chunks<II>(chunk_size: usize, iter: II) -> ChunksAsTuple<I>
    where
        II: IntoIterator<IntoIter = I>,
    {
        ChunksAsTuple {
            chunk_size,
            iter: iter.into_iter(),
        }
    }
}

impl<'args, DB, I> SqlxPushAsTuple<'args, DB> for IterAsTuple<I>
where
    DB: Database,
    I: Iterator,
    I::Item: 'args + Encode<'args, DB> + Send + Type<DB>,
{
    fn push_as_inner_tuple(self, builder: &mut QueryBuilder<'args, DB>) {
        let mut separated = builder.separated(", ");
        separated.push_bind(self.first);
        for value in self.iter {
            separated.push_bind(value);
        }
    }
}

pub(super) struct ChunksAsTuple<I>
where
    I: Iterator,
{
    chunk_size: usize,
    iter: I,
}

impl<I> ChunksAsTuple<I>
where
    I: Iterator,
{
    pub(super) fn next(&mut self) -> Option<IterAsTuple<iter::Take<&mut I>>> {
        IterAsTuple::new(self.iter.by_ref().take(self.chunk_size)).ok()
    }

    pub(super) fn element_count(&self) -> usize
    where
        I: ExactSizeIterator,
    {
        self.iter.len()
    }
}

#[derive(Copy, Clone, Debug, Type)]
#[sqlx(transparent)]
pub(super) struct SqlBitCastU32(i32);

impl From<u32> for SqlBitCastU32 {
    fn from(value: u32) -> Self {
        #![allow(clippy::cast_possible_wrap)]
        Self(value as i32)
    }
}

impl From<SqlBitCastU32> for u32 {
    fn from(value: SqlBitCastU32) -> Self {
        #![allow(clippy::cast_sign_loss)]
        value.0 as u32
    }
}

#[cfg(test)]
mod tests {
    use sqlx::Postgres;

    use super::*;

    #[test]
    fn test_iter_as_tuple_ensures_non_empty() {
        assert!(IterAsTuple::new(Vec::<i32>::new()).is_err());
        assert!(IterAsTuple::new(vec![1]).is_ok());
    }

    #[test]
    fn test_push_tuple() {
        let mut builder = QueryBuilder::<Postgres>::new("-- ");
        builder.push_tuple(IterAsTuple::new(vec!["a"]).unwrap());
        assert_eq!(builder.sql(), "-- ($1)");

        builder.reset();
        builder.push_tuple(IterAsTuple::new(vec![1, 2, 3]).unwrap());
        assert_eq!(builder.sql(), "-- ($1, $2, $3)");
    }

    #[test]
    fn test_push_nested_tuple() {
        let mut builder = QueryBuilder::<Postgres>::new("-- ");
        builder.push_nested_tuple(IterAsTuple::new(vec![(1,)]).unwrap());
        assert_eq!(builder.sql(), "-- (($1))");

        builder.reset();
        builder.push_nested_tuple(IterAsTuple::new(vec![(1, 1)]).unwrap());
        assert_eq!(builder.sql(), "-- (($1, $2))");

        builder.reset();
        builder.push_nested_tuple(IterAsTuple::new(vec![(1,), (1,)]).unwrap());
        assert_eq!(builder.sql(), "-- (($1), ($2))");

        builder.reset();
        builder.push_nested_tuple(IterAsTuple::new(vec![(1, 1), (1, 1)]).unwrap());
        assert_eq!(builder.sql(), "-- (($1, $2), ($3, $4))");

        builder.reset();
        builder.push_nested_tuple(
            IterAsTuple::new(vec![
                IterAsTuple::new(vec![1, 1]).unwrap(),
                IterAsTuple::new(vec![1, 1]).unwrap(),
            ])
            .unwrap(),
        );
        assert_eq!(builder.sql(), "-- (($1, $2), ($3, $4))");
    }

    #[test]
    fn test_chunking_is_done_appropriately() {
        let mut builder = QueryBuilder::<Postgres>::new("-- ");
        let mut iter = IterAsTuple::chunks(3, vec![1, 1, 1, 2, 2, 2, 3]);
        while let Some(chunk) = iter.next() {
            //Hint: normally you would reset the builder in each iteration
            builder.push_tuple(chunk);
        }
        assert_eq!(builder.sql(), "-- ($1, $2, $3)($4, $5, $6)($7)");

        builder.reset();
        let mut iter = IterAsTuple::chunks(3, vec![1, 1]);
        while let Some(chunk) = iter.next() {
            //Hint: normally you would reset the builder in each iteration
            builder.push_tuple(chunk);
        }
        assert_eq!(builder.sql(), "-- ($1, $2)");

        builder.reset();
        let mut iter = IterAsTuple::chunks(3, Vec::<i32>::new());
        while let Some(chunk) = iter.next() {
            //Hint: normally you would reset the builder in each iteration
            builder.push_tuple(chunk);
        }
        assert_eq!(builder.sql(), "-- ");
    }

    #[test]
    fn test_sql_bitcast_u32() {
        assert_eq!(u32::from(SqlBitCastU32::from(u32::MAX)), u32::MAX);
        assert_eq!(u32::from(SqlBitCastU32::from(100)), 100);
        assert_eq!(SqlBitCastU32::from(u32::MAX).0, -1);
    }
}
