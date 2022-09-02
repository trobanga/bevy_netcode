use std::collections::HashMap;

use tracing::info;

use crate::helper::TestAppBuilder;

#[derive(Debug, serde::Deserialize)]
struct User {
    username: String,
}

#[actix_web::test]
async fn add_user() {
    let mut app = TestAppBuilder::new().build();
    app.spawn_app().await;
    let path = app.generate_path("user/add");
    let mut map = HashMap::new();
    map.insert("username", "Alice");
    map.insert("pwd", "I like Bob");

    let client = reqwest::Client::new();
    let response = client.post(&path).json(&map).send().await.unwrap();
    assert_eq!(response.status(), 200);

    let user = client
        .get(app.generate_path("user/Alice"))
        .send()
        .await
        .unwrap();

    let user = user.text().await.unwrap();
    let user: User = serde_json::from_str(&user).unwrap();
    assert_eq!(user.username, "Alice".to_string());
}

#[actix_web::test]
async fn delete_user() {
    let mut app = TestAppBuilder::new().with_default_user_alice().build();
    app.spawn_app().await;

    let client = reqwest::Client::new();
    let user = client
        .get(app.generate_path("user/Alice"))
        .basic_auth("Alice", Some("I like Bob"))
        .send()
        .await
        .unwrap();

    let user = user.text().await.unwrap();
    let user: User = serde_json::from_str(&user).unwrap();
    assert_eq!(user.username, "Alice".to_string());

    let mut map = Vec::new();
    map.push("Alice".to_string());
    let path = app.generate_path("user/del/Alice");
    let response = client
        .delete(&path)
        .basic_auth("Alice", Some("I like Bob"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    let users = client.get(app.generate_path("users")).send().await.unwrap();
    let users = users.text().await.unwrap();
    let users: Vec<String> = serde_json::from_str(&users).unwrap();

    info!(?users);
    assert_eq!(users.len(), 0);
}

#[actix_web::test]
async fn cannot_add_user_if_name_is_taken() {
    let mut app = TestAppBuilder::new().with_default_user_alice().build();
    app.spawn_app().await;
    let path = app.generate_path("user/add");
    let mut map = HashMap::new();
    map.insert("username", "Alice");
    map.insert("pwd", "I like Bob");

    let client = reqwest::Client::new();
    let response = client.post(&path).json(&map).send().await.unwrap();
    assert_eq!(response.status(), 500);

    let users = client.get(app.generate_path("users")).send().await.unwrap();
    let users = users.text().await.unwrap();
    let users: Vec<String> = serde_json::from_str(&users).unwrap();

    assert_eq!(users.len(), 1);
}
