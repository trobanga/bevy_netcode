use crate::{
    helper::{enable_tracing, TestAppBuilder, TestUser},
    test_db::TestDb,
};
use matchmaker::{
    authentication::{validate_credentials, Credentials},
    db::{self, actions::find_user_by_name},
};
use secrecy::Secret;
#[actix_web::test]
async fn password_hashed() {
    enable_tracing();
    let test_db = TestDb::new();
    let pool = db::create_pool(test_db.url());
    let mut conn = test_db.conn();
    test_db.run_migrations(&mut conn).unwrap();

    let user = TestUser::default();
    user.store(&pool);

    let user = find_user_by_name("Alice", &mut conn).unwrap().unwrap();

    assert_ne!(user.password, "I like Bob");
}

#[actix_web::test]
async fn password_verification() {
    enable_tracing();
    let test_db = TestDb::new();
    let pool = db::create_pool(test_db.url());
    let mut conn = test_db.conn();
    test_db.run_migrations(&mut conn).unwrap();

    let user = TestUser::default();
    user.store(&pool);

    let credentials = Credentials {
        username: "Alice".to_string(),
        password: Secret::new("I like Bob".to_string()),
    };

    match validate_credentials(credentials, &mut conn).await {
        Ok(_) => {}
        Err(_) => panic!("Password does not match"),
    }
}

#[actix_web::test]
async fn missing_auth_are_rejected_with_reqwest() -> anyhow::Result<()> {
    let mut app = TestAppBuilder::new().build();
    app.spawn_app().await;
    let address = format!("http://{}:{}/", &app.address, app.port);
    let response = reqwest::Client::new().get(&address).send().await?;
    assert_eq!(response.status(), 401);
    Ok(())
}

#[actix_web::test]
async fn wrong_auth_are_rejected_with_reqwest() -> anyhow::Result<()> {
    let mut app = TestAppBuilder::new().build();
    app.spawn_app().await;
    let address = format!("http://{}:{}/", &app.address, app.port);
    let response = reqwest::Client::new()
        .get(&address)
        .basic_auth("Alice", Some("I don't like Bob"))
        .send()
        .await?;
    assert_eq!(response.status(), 401);

    let response = reqwest::Client::new()
        .get(&address)
        .basic_auth("Malice", Some("I like Bob"))
        .send()
        .await?;
    assert_eq!(response.status(), 401);
    Ok(())
}

#[actix_web::test]
async fn right_auth_pass_with_reqwest_yield_bad_request() -> anyhow::Result<()> {
    let mut app = TestAppBuilder::new().build();
    app.spawn_app().await;
    let address = format!("http://{}:{}/", &app.address, app.port);
    let response = reqwest::Client::new()
        .get(&address)
        .basic_auth("Alice", Some("I like Bob"))
        .send()
        .await?;
    assert_eq!(response.status(), 400);
    Ok(())
}

#[actix_web::test]
async fn correct_auth_are_redirected() -> anyhow::Result<()> {
    let mut app = TestAppBuilder::new().build();
    app.spawn_app().await;
    let address = format!("ws://{}:{}/", app.address, app.port);
    let (res, _ws) = client::Client::connect(&address, "Alice", "I like Bob").await?;

    assert_eq!(res.status(), 101);
    Ok(())
}
