use diesel::prelude::*;
use secrecy::{ExposeSecret, Secret};

use super::models;
use super::schema;
use super::DbPool;

pub fn create_user(
    username: &str,
    pwd: Secret<String>,
    pool: &DbPool,
) -> Result<(), anyhow::Error> {
    use schema::users::dsl::*;

    let mut conn = pool.get()?;

    let new_user = models::NewUser {
        name: username,
        password: pwd.expose_secret(),
    };

    diesel::insert_into(users)
        .values(&new_user)
        .execute(&mut conn)?;

    Ok(())
}

pub fn find_user_by_name(uname: &str, pool: &DbPool) -> Result<models::User, anyhow::Error> {
    use schema::users::dsl::*;

    let mut conn = pool.get()?;
    let user = users
        .filter(name.eq(uname.to_string()))
        .first::<models::User>(&mut conn)?;

    Ok(user)
}

pub fn set_password_for_user(
    uid: uuid::Uuid,
    new_password: Secret<String>,
    pool: &DbPool,
) -> Result<(), anyhow::Error> {
    use schema::users::dsl::*;

    let mut conn = pool.get()?;
    diesel::update(users.find(uid))
        .set(password.eq(new_password.expose_secret()))
        .execute(&mut conn)?;
    Ok(())
}
