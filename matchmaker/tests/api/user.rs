use std::collections::HashMap;

use matchmaker::db::actions::display_users;
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
    let path = app.generate_path("add_user");
    let mut map = HashMap::new();
    map.insert("username", "Alice");
    map.insert("pwd", "I like Bob");

    let client = reqwest::Client::new();
    let request = client.post(&path).json(&map);
    info!(?request);
    let response = request.send().await.unwrap();

    assert_eq!(response.status(), 200);
    let mut conn = app.db_pool.get().unwrap();
    info!("Users:");
    display_users(&mut conn).unwrap();

    let user = client
        .get(app.generate_path("user/Alice"))
        .send()
        .await
        .unwrap();

    let user = user.text().await.unwrap();
    let user: User = serde_json::from_str(&user).unwrap();
    info!(?user);

    assert!(false);
}
