use clinvoice_adapter::{schema::columns::LocationColumns, Deletable};
use clinvoice_schema::{Id, Location};
use sqlx::{Executor, Postgres, Result};

use super::PgLocation;
use crate::PgSchema;

#[async_trait::async_trait]
impl Deletable for PgLocation
{
	type Db = Postgres;
	type Entity = Location;

	async fn delete<'connection, 'entity, Conn, Iter>(
		connection: Conn,
		entities: Iter,
	) -> Result<()>
	where
		Self::Entity: 'entity,
		Conn: Executor<'connection, Database = Self::Db>,
		Iter: Iterator<Item = &'entity Self::Entity> + Send,
	{
		const fn mapper(l: &Location) -> Id
		{
			l.id
		}

		// TODO: use `for<'a> |e: &'a Location| e.id`
		PgSchema::delete::<_, _, LocationColumns<char>>(connection, entities.map(mapper)).await
	}
}

#[cfg(test)]
mod tests
{
	use clinvoice_adapter::{schema::LocationAdapter, Deletable, Retrievable};
	use clinvoice_match::Match;
	use pretty_assertions::assert_eq;

	use crate::schema::{util, PgLocation};

	#[tokio::test]
	async fn delete()
	{
		let connection = util::connect().await;

		let earth = PgLocation::create(&connection, "Earth".into(), None).await.unwrap();

		let (chile, usa) = futures::try_join!(
			PgLocation::create(&connection, "Chile".into(), Some(earth.clone())),
			PgLocation::create(&connection, "Arizona".into(), Some(earth.clone())),
		)
		.unwrap();

		assert!(PgLocation::delete(&connection, [&earth].into_iter()).await.is_err());
		PgLocation::delete(&connection, [&chile, &usa].into_iter()).await.unwrap();

		assert_eq!(
			PgLocation::retrieve(
				&connection,
				Match::Or(vec![chile.id.into(), earth.id.into(), usa.id.into(),]).into(),
			)
			.await
			.unwrap()
			.as_slice(),
			&[earth]
		);
	}
}
