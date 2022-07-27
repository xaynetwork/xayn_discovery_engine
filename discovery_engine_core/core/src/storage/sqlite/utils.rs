use sqlx::{Database, Encode, QueryBuilder, Type};

pub(super) trait SqlxPushTupleExt<'args, DB: Database> {
    fn push_tuple<I>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: 'args + Encode<'args, DB> + Send + Type<DB>;
}

impl<'args, DB> SqlxPushTupleExt<'args, DB> for QueryBuilder<'args, DB>
where
    DB: Database,
{
    fn push_tuple<I>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: 'args + Encode<'args, DB> + Send + Type<DB>,
    {
        let mut separated = self.push("(").separated(", ");
        for value in values {
            separated.push_bind(value);
        }
        separated.push_unseparated(")");
        drop(separated);
        self
    }
}
