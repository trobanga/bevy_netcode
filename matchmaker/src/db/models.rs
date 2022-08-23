use super::schema::users;

#[derive(Queryable, Debug)]
pub struct User {
    pub uuid: uuid::Uuid,
    pub name: String,
    pub password: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub uuid: uuid::Uuid,
    pub name: &'a str,
    pub password: String,
}
