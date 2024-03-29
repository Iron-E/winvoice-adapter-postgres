mod deletable;
mod organization_adapter;
mod retrievable;
mod updatable;

use clinvoice_adapter::schema::columns::OrganizationColumns;
use clinvoice_schema::Organization;
use sqlx::{postgres::PgRow, Executor, Postgres, Result, Row};

use super::PgLocation;

/// Implementor of the [`OrganizationAdapter`](clinvoice_adapter::schema::OrganizationAdapter) for
/// the [`Postgres`](sqlx::Postgres) database.
pub struct PgOrganization;

impl PgOrganization
{
	pub(super) async fn row_to_view<'connection, Conn, Column>(
		connection: Conn,
		columns: OrganizationColumns<Column>,
		row: &PgRow,
	) -> Result<Organization>
	where
		Conn: Executor<'connection, Database = Postgres>,
		Column: AsRef<str>,
	{
		let location_id = row.try_get(columns.location_id.as_ref())?;
		Ok(Organization {
			id: row.try_get(columns.id.as_ref())?,
			name: row.try_get(columns.name.as_ref())?,
			location: PgLocation::retrieve_by_id(connection, location_id).await?,
		})
	}
}
