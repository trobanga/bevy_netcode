use super::schema::users;
use std::fmt;

#[derive(Queryable, Debug)]
pub struct User {
    pub uuid: uuid::Uuid,
    pub name: String,
    pub password: String,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User: {}: {}", self.name, self.uuid)
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub uuid: uuid::Uuid,
    pub name: &'a str,
    pub password: String,
}
