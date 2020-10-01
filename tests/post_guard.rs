use async_graphql::guard::PostGuard;
use async_graphql::*;

#[derive(Eq, PartialEq, Copy, Clone)]
enum Role {
    Admin,
    Guest,
}

struct RoleGuard {
    role: Role,
}

#[async_trait::async_trait]
impl PostGuard<i32> for RoleGuard {
    async fn check(&self, ctx: &Context<'_>, _result: &i32) -> Result<()> {
        if ctx.data_opt::<Role>() == Some(&self.role) {
            Ok(())
        } else {
            Err("Forbidden".into())
        }
    }
}

#[derive(SimpleObject)]
struct MyObj {
    #[graphql(owned, post_guard(UserGuard(username = r#""test""#, value = "88")))]
    value: i32,
}

struct Username(String);

struct UserGuard {
    value: i32,
    username: String,
}

#[async_trait::async_trait]
impl PostGuard<i32> for UserGuard {
    async fn check(&self, ctx: &Context<'_>, result: &i32) -> Result<()> {
        assert_eq!(*result, self.value);
        if ctx.data_opt::<Username>().as_ref().map(|s| s.0.as_str()) == Some(&self.username) {
            Ok(())
        } else {
            Err("Forbidden".into())
        }
    }
}

#[async_trait::async_trait]
impl PostGuard<MyObj> for UserGuard {
    async fn check(&self, ctx: &Context<'_>, result: &MyObj) -> Result<()> {
        assert_eq!(result.value, self.value);
        if ctx.data_opt::<Username>().as_ref().map(|s| s.0.as_str()) == Some(&self.username) {
            Ok(())
        } else {
            Err("Forbidden".into())
        }
    }
}

#[async_std::test]
pub async fn test_post_guard() {
    struct Query;

    #[Object]
    impl Query {
        #[graphql(post_guard(UserGuard(username = r#""test""#, value = "99")))]
        async fn value(&self) -> i32 {
            99
        }

        async fn obj(&self) -> MyObj {
            MyObj { value: 88 }
        }
    }

    let schema = Schema::new(Query, EmptyMutation, EmptySubscription);

    let query = "{ value }";
    assert_eq!(
        schema
            .execute(Request::new(query).data(Username("test".to_string())))
            .await
            .data,
        serde_json::json!({
            "value": 99
        })
    );

    let query = "{ value }";
    assert_eq!(
        schema
            .execute(Request::new(query).data(Username("test1".to_string())))
            .await
            .into_result()
            .unwrap_err(),
        vec![ServerError {
            message: "Forbidden".to_string(),
            locations: vec![Pos { line: 1, column: 3 }],
            path: vec![PathSegment::Field("value".to_owned())],
            extensions: None,
        }]
    );

    let query = "{ obj { value } }";
    assert_eq!(
        schema
            .execute(Request::new(query).data(Username("test".to_string())))
            .await
            .data,
        serde_json::json!({
            "obj": { "value": 88 }
        })
    );

    let query = "{ obj { value } }";
    assert_eq!(
        schema
            .execute(Request::new(query).data(Username("test1".to_string())))
            .await
            .into_result()
            .unwrap_err(),
        vec![ServerError {
            message: "Forbidden".to_string(),
            locations: vec![Pos { line: 1, column: 9 }],
            path: vec![
                PathSegment::Field("obj".to_owned()),
                PathSegment::Field("value".to_owned())
            ],
            extensions: None,
        }]
    );
}

#[async_std::test]
pub async fn test_multiple_post_guards() {
    #[derive(SimpleObject)]
    struct Query {
        #[graphql(post_guard(
            RoleGuard(role = "Role::Admin"),
            UserGuard(username = r#""test""#, value = "10")
        ))]
        value: i32,
    }

    let schema = Schema::new(Query { value: 10 }, EmptyMutation, EmptySubscription);

    let query = "{ value }";
    assert_eq!(
        schema
            .execute(
                Request::new(query)
                    .data(Role::Admin)
                    .data(Username("test".to_string()))
            )
            .await
            .data,
        serde_json::json!({"value": 10})
    );

    let query = "{ value }";
    assert_eq!(
        schema
            .execute(
                Request::new(query)
                    .data(Role::Guest)
                    .data(Username("test".to_string()))
            )
            .await
            .into_result()
            .unwrap_err(),
        vec![ServerError {
            message: "Forbidden".to_string(),
            locations: vec![Pos { line: 1, column: 3 }],
            path: vec![PathSegment::Field("value".to_owned())],
            extensions: None,
        }]
    );

    let query = "{ value }";
    assert_eq!(
        schema
            .execute(
                Request::new(query)
                    .data(Role::Admin)
                    .data(Username("test1".to_string()))
            )
            .await
            .into_result()
            .unwrap_err(),
        vec![ServerError {
            message: "Forbidden".to_string(),
            locations: vec![Pos { line: 1, column: 3 }],
            path: vec![PathSegment::Field("value".to_owned())],
            extensions: None,
        }]
    );

    let query = "{ value }";
    assert_eq!(
        schema
            .execute(
                Request::new(query)
                    .data(Role::Guest)
                    .data(Username("test1".to_string()))
            )
            .await
            .into_result()
            .unwrap_err(),
        vec![ServerError {
            message: "Forbidden".to_string(),
            locations: vec![Pos { line: 1, column: 3 }],
            path: vec![PathSegment::Field("value".to_owned())],
            extensions: None,
        }]
    );
}

#[async_std::test]
pub async fn test_post_guard_forward_arguments() {
    struct UserGuard {
        id: ID,
    }

    #[async_trait::async_trait]
    impl PostGuard<ID> for UserGuard {
        async fn check(&self, ctx: &Context<'_>, result: &ID) -> Result<()> {
            assert_eq!(result.as_str(), "haha");
            if ctx.data_opt::<ID>() != Some(&self.id) {
                Err("Forbidden".into())
            } else {
                Ok(())
            }
        }
    }

    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        #[graphql(post_guard(UserGuard(id = "@_id")))]
        async fn user(&self, _id: ID) -> ID {
            "haha".into()
        }
    }

    let schema = Schema::new(QueryRoot, EmptyMutation, EmptySubscription);

    let query = r#"{ user(id: "abc") }"#;
    assert_eq!(
        schema
            .execute(Request::new(query).data(ID::from("abc")))
            .await
            .data,
        serde_json::json!({"user": "haha"})
    );

    let query = r#"{ user(id: "abc") }"#;
    assert_eq!(
        schema
            .execute(Request::new(query).data(ID::from("aaa")))
            .await
            .into_result()
            .unwrap_err(),
        vec![ServerError {
            message: "Forbidden".to_string(),
            locations: vec![Pos { line: 1, column: 3 }],
            path: vec![PathSegment::Field("user".to_owned())],
            extensions: None,
        }]
    );
}

#[async_std::test]
pub async fn test_post_guard_generic() {
    struct UserGuard {
        id: ID,
    }

    #[async_trait::async_trait]
    impl<T: Send + Sync> PostGuard<T> for UserGuard {
        async fn check(&self, ctx: &Context<'_>, _result: &T) -> Result<()> {
            if ctx.data_opt::<ID>() != Some(&self.id) {
                Err("Forbidden".into())
            } else {
                Ok(())
            }
        }
    }

    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        #[graphql(post_guard(UserGuard(id = r#""abc""#)))]
        async fn user(&self) -> ID {
            "haha".into()
        }
    }

    let schema = Schema::new(QueryRoot, EmptyMutation, EmptySubscription);

    let query = r#"{ user }"#;
    assert_eq!(
        schema
            .execute(Request::new(query).data(ID::from("abc")))
            .await
            .data,
        serde_json::json!({"user": "haha"})
    );

    let query = r#"{ user }"#;
    assert_eq!(
        schema
            .execute(Request::new(query).data(ID::from("aaa")))
            .await
            .into_result()
            .unwrap_err(),
        vec![ServerError {
            message: "Forbidden".to_string(),
            locations: vec![Pos { line: 1, column: 3 }],
            path: vec![PathSegment::Field("user".to_owned())],
            extensions: None,
        }]
    );
}
