use clinvoice_adapter::{schema::columns::ContactColumns, Updatable};
use clinvoice_schema::Contact;
use sqlx::{Postgres, Result, Transaction};

use super::PgContact;
use crate::PgSchema;

#[async_trait::async_trait]
impl Updatable for PgContact
{
	type Db = Postgres;
	type Entity = Contact;

	async fn update<'entity, Iter>(
		connection: &mut Transaction<Self::Db>,
		entities: Iter,
	) -> Result<()>
	where
		Self::Entity: 'entity,
		Iter: Clone + Iterator<Item = &'entity Self::Entity> + Send,
	{
		let mut peekable_entities = entities.peekable();

		// There is nothing to do.
		if peekable_entities.peek().is_none()
		{
			return Ok(());
		}

		PgSchema::update(connection, ContactColumns::default(), |query| {
			query.push_values(peekable_entities, |mut q, e| {
				q.push_bind(e.kind.address().map(|a| a.id))
					.push_bind(e.kind.email())
					.push_bind(&e.label)
					.push_bind(e.kind.other())
					.push_bind(e.kind.phone());
			});
		})
		.await
	}
}

#[cfg(test)]
mod tests
{
	use std::collections::HashSet;

	use clinvoice_adapter::{
		schema::{ContactAdapter, LocationAdapter},
		Deletable,
		Retrievable,
		Updatable,
	};
	use clinvoice_match::{MatchContact, MatchStr};
	use clinvoice_schema::ContactKind;
	use pretty_assertions::assert_eq;

	use crate::schema::{util, PgContact, PgLocation};

	#[tokio::test]
	async fn update()
	{
		let connection = util::connect().await;

		let (earth, mars) = futures::try_join!(
			PgLocation::create(&connection, "Earth".into(), None),
			PgLocation::create(&connection, "Mars".into(), None),
		)
		.unwrap();

		let (mut office, mut phone) = futures::try_join!(
			PgContact::create(
				&connection,
				ContactKind::Address(earth),
				"asldkjalskfhalskdj Office".into()
			),
			PgContact::create(
				&connection,
				ContactKind::Phone("1-800-555-5555".into()),
				"gbtyufs buai Primary Contact".into()
			),
		)
		.unwrap();

		office.kind = ContactKind::Address(mars);
		phone.kind = ContactKind::Email("foo@bar.io".into());

		{
			let mut transaction = connection.begin().await.unwrap();
			PgContact::update(&mut transaction, [&office, &phone].into_iter()).await.unwrap();
			transaction.commit().await.unwrap();
		}

		let db_contact_info: HashSet<_> = PgContact::retrieve(&connection, MatchContact {
			label: MatchStr::Or(vec![office.label.clone().into(), phone.label.clone().into()]),
			..Default::default()
		})
		.await
		.unwrap()
		.into_iter()
		.collect();

		assert_eq!([&office, &phone].into_iter().cloned().collect::<HashSet<_>>(), db_contact_info);

		// cleanup
		PgContact::delete(&connection, [&office, &phone].into_iter()).await.unwrap();
	}
}
