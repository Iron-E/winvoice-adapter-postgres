use clinvoice_adapter::schema::{ExpensesAdapter, TimesheetAdapter};
use clinvoice_schema::{
	chrono::{DateTime, Utc},
	Employee,
	Job,
	Timesheet,
};
use money2::Money;
use sqlx::{Postgres, Result, Transaction};

use super::PgTimesheet;
use crate::{fmt::DateTimeExt, schema::PgExpenses};

#[async_trait::async_trait]
impl TimesheetAdapter for PgTimesheet
{
	async fn create(
		connection: &mut Transaction<Postgres>,
		employee: Employee,
		expenses: Vec<(String, Money, String)>,
		job: Job,
		time_begin: DateTime<Utc>,
		time_end: Option<DateTime<Utc>>,
		work_notes: String,
	) -> Result<Timesheet>
	{
		let row = sqlx::query!(
			"INSERT INTO timesheets
				(employee_id, job_id, time_begin, time_end, work_notes)
			VALUES
				($1,          $2,     $3,         $4,       $5)
			RETURNING id;",
			employee.id,
			job.id,
			time_begin,
			time_end,
			work_notes,
		)
		.fetch_one(&mut *connection)
		.await?;

		let expenses_db = PgExpenses::create(connection, expenses, row.id).await?;

		Ok(Timesheet {
			id: row.id,
			employee,
			expenses: expenses_db,
			job,
			time_begin,
			time_end,
			work_notes,
		}
		.pg_sanitize())
	}
}

#[cfg(test)]
mod tests
{
	use core::time::Duration;

	use clinvoice_adapter::schema::{
		EmployeeAdapter,
		JobAdapter,
		LocationAdapter,
		OrganizationAdapter,
	};
	use clinvoice_schema::{
		chrono::{TimeZone, Utc},
		Currency,
		Invoice,
		InvoiceDate,
		Money,
	};
	use pretty_assertions::assert_eq;

	use super::{PgTimesheet, TimesheetAdapter};
	use crate::schema::{util, PgEmployee, PgJob, PgLocation, PgOrganization};

	#[tokio::test]
	async fn retrieve()
	{
		let connection = util::connect().await;

		let earth = PgLocation::create(&connection, "Earth".into(), None).await.unwrap();

		let usa = PgLocation::create(&connection, "USA".into(), Some(earth)).await.unwrap();

		let (arizona, utah) = futures::try_join!(
			PgLocation::create(&connection, "Arizona".into(), Some(usa.clone())),
			PgLocation::create(&connection, "Utah".into(), Some(usa.clone())),
		)
		.unwrap();

		let (organization, organization2) = futures::try_join!(
			PgOrganization::create(&connection, arizona.clone(), "Some Organization".into()),
			PgOrganization::create(&connection, utah, "Some Other Organizatión".into()),
		)
		.unwrap();

		let (employee, employee2) = futures::try_join!(
			PgEmployee::create(&connection, "My Name".into(), "Employed".into(), "Janitor".into()),
			PgEmployee::create(
				&connection,
				"Another Gúy".into(),
				"Management".into(),
				"Assistant to Regional Manager".into(),
			),
		)
		.unwrap();

		let (job, job2) = futures::try_join!(
			PgJob::create(
				&connection,
				organization.clone(),
				None,
				Utc.ymd(1990, 07, 12).and_hms(14, 10, 00),
				Duration::from_secs(900),
				Invoice { date: None, hourly_rate: Money::new(20_00, 2, Currency::Usd) },
				String::new(),
				"Do something".into()
			),
			PgJob::create(
				&connection,
				organization2.clone(),
				Some(Utc.ymd(3000, 01, 13).and_hms(11, 30, 00)),
				Utc.ymd(3000, 01, 12).and_hms(09, 15, 42),
				Duration::from_secs(900),
				Invoice {
					date: Some(InvoiceDate {
						issued: Utc.ymd(3000, 01, 13).and_hms(11, 45, 00),
						paid:   Some(Utc.ymd(3000, 01, 15).and_hms(14, 27, 00)),
					}),
					hourly_rate: Money::new(200_00, 2, Currency::Jpy),
				},
				String::new(),
				"Do something".into()
			),
		)
		.unwrap();

		// {{{
		let mut transaction = connection.begin().await.unwrap();

		let timesheet = PgTimesheet::create(
			&mut transaction,
			employee,
			Vec::new(),
			job,
			Utc::now(),
			None,
			"This is my work notes".into(),
		)
		.await
		.unwrap();

		let timesheet2 = PgTimesheet::create(
			&mut transaction,
			employee2,
			vec![(
				"Flight".into(),
				Money::new(300_56, 2, Currency::Usd),
				"Trip to Hawaii for research".into(),
			)],
			job2,
			Utc.ymd(2022, 06, 08).and_hms(15, 27, 00),
			Some(Utc.ymd(2022, 06, 09).and_hms(07, 00, 00)),
			"This is more work notes".into(),
		)
		.await
		.unwrap();

		transaction.commit().await.unwrap();
		// }}}

		macro_rules! select {
			($id:expr) => {
				sqlx::query!("SELECT * FROM timesheets WHERE id = $1", $id)
					.fetch_one(&connection)
					.await
					.unwrap()
			};
		}

		let timesheet_db = select!(timesheet.id);
		assert_eq!(timesheet_db.employee_id, timesheet.employee.id);
		assert_eq!(timesheet_db.id, timesheet.id);
		assert_eq!(timesheet_db.job_id, timesheet.job.id);
		assert_eq!(timesheet_db.time_begin, timesheet.time_begin);
		assert_eq!(timesheet_db.time_end, timesheet.time_end);
		assert_eq!(timesheet_db.work_notes, timesheet.work_notes);

		let timesheet2_db = select!(timesheet2.id);
		assert_eq!(timesheet2_db.employee_id, timesheet2.employee.id);
		assert_eq!(timesheet2_db.id, timesheet2.id);
		assert_eq!(timesheet2_db.job_id, timesheet2.job.id);
		assert_eq!(timesheet2_db.time_begin, timesheet2.time_begin);
		assert_eq!(timesheet2_db.time_end, timesheet2.time_end);
		assert_eq!(timesheet2_db.work_notes, timesheet2.work_notes);
	}
}
