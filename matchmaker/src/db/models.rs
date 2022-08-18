use super::schema::users;

#[derive(Queryable)]
pub struct User {
    pub uuid: uuid::Uuid,
    pub name: String,
    pub password: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub password: &'a str,
}
