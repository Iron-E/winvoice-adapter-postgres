use clinvoice_adapter::{
	fmt::{sql, QueryBuilderExt, TableToSql},
	schema::columns::EmployeeColumns,
	Retrievable,
	WriteWhereClause,
};
use clinvoice_match::MatchEmployee;
use clinvoice_schema::Employee;
use futures::TryStreamExt;
use sqlx::{Pool, Postgres, QueryBuilder, Result};

use super::PgEmployee;
use crate::PgSchema;

/// Implementors of this trait are capable of being retrieved from a [`Database`].
#[async_trait::async_trait]
impl Retrievable for PgEmployee
{
	/// The [`Database`] where data of type [`Updatable::Entity`] is being stored.
	type Db = Postgres;
	/// The type of data that is to be [`update`](Deletable::update)d.
	type Entity = Employee;
	/// The type used for [match](clinvoice_match)ing.
	type Match = MatchEmployee;

	/// Retrieve all [`Employee`]s (via `connection`) that match the `match_condition`.
	async fn retrieve(
		connection: &Pool<Postgres>,
		match_condition: Self::Match,
	) -> Result<Vec<Self::Entity>>
	{
		const COLUMNS: EmployeeColumns<&'static str> = EmployeeColumns::default();

		let mut query = QueryBuilder::new(sql::SELECT);

		query.push_columns(&COLUMNS.default_scope()).push_default_from::<EmployeeColumns<char>>();

		PgSchema::write_where_clause(
			Default::default(),
			EmployeeColumns::<char>::DEFAULT_ALIAS,
			&match_condition,
			&mut query,
		);

		query
			.prepare()
			.fetch(connection)
			.map_ok(|row| Self::row_to_view(COLUMNS, &row))
			.try_collect()
			.await
	}
}

#[cfg(test)]
mod tests
{
	use std::collections::HashSet;

	use clinvoice_adapter::{schema::EmployeeAdapter, Retrievable};
	use clinvoice_match::{Match, MatchEmployee, MatchStr};

	use crate::schema::{util, PgEmployee};

	#[tokio::test]
	async fn retrieve()
	{
		let connection = util::connect().await;

		let (employee, employee2) = futures::try_join!(
			PgEmployee::create(&connection, "My Name".into(), "Employed".into(), "Janitor".into(),),
			PgEmployee::create(
				&connection,
				"Another Gúy".into(),
				"Management".into(),
				"Assistant to Regional Manager".into(),
			),
		)
		.unwrap();

		assert_eq!(
			PgEmployee::retrieve(&connection, MatchEmployee {
				id: Match::Or(vec![employee.id.into(), employee2.id.into()]),
				name: employee.name.clone().into(),
				..Default::default()
			})
			.await
			.unwrap()
			.as_slice(),
			&[employee.clone()],
		);

		assert_eq!(
			PgEmployee::retrieve(&connection, MatchEmployee {
				id: Match::Or(vec![employee.id.into(), employee2.id.into()]),
				name: MatchStr::Not(MatchStr::from("Fired".to_string()).into()),
				..Default::default()
			})
			.await
			.unwrap()
			.into_iter()
			.collect::<HashSet<_>>(),
			[employee, employee2].into_iter().collect()
		);
	}
}
