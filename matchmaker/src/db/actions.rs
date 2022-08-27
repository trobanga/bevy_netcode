use diesel::prelude::*;
use secrecy::{ExposeSecret, Secret};
use tracing::info;
use uuid::Uuid;

use crate::authentication::compute_password_hash;

use super::models;
use super::schema;
use super::DbConnection;

pub fn create_user(
    username: &str,
    pwd: Secret<String>,
    conn: &mut DbConnection,
) -> Result<(), anyhow::Error> {
    use schema::users::dsl::*;

    let pwd = compute_password_hash(pwd)?;
    let new_user = models::NewUser {
        uuid: Uuid::new_v4(),
        name: username,
        password: pwd.expose_secret().to_string(),
    };

    diesel::insert_into(users).values(&new_user).execute(conn)?;

    Ok(())
}

pub fn find_user_by_name(
    uname: &str,
    conn: &mut DbConnection,
) -> Result<Option<models::User>, anyhow::Error> {
    use schema::users::dsl::*;
    let user = users
        .filter(name.eq(uname.to_string()))
        .first::<models::User>(conn)
        .optional()?;

    Ok(user)
}

pub fn set_password_for_user(
    uid: uuid::Uuid,
    new_password: Secret<String>,
    conn: &mut DbConnection,
) -> Result<(), anyhow::Error> {
    use schema::users::dsl::*;

    diesel::update(users.find(uid))
        .set(password.eq(new_password.expose_secret()))
        .execute(conn)?;
    Ok(())
}

pub fn display_users(conn: &mut DbConnection) -> Result<(), anyhow::Error> {
    use schema::users::dsl::*;
    for user in users.load::<models::User>(conn)? {
        info!("{user}");
    }

    Ok(())
}
