# Skill: Rust Integration Testing for Nebripop

Esta skill define el estándar para asegurar la calidad de Nebripop mediante tests de integración robustos utilizando `sqlx::test`, `mockall` y el ecosistema de testing de Axum.

## Contexto
Según el **PRD (Sección 10)**, la excelencia del proyecto requiere al menos un test de integración por cada uno de los 8 módulos Must Have. Se prioriza el uso de bases de datos efímeras para evitar efectos secundarios entre tests.

## Reglas y Ejemplos

### 1. Uso de `sqlx::test` para DB Efímera
Utiliza el atributo `#[sqlx::test]` para que cada test se ejecute en una base de datos limpia y aislada.

```rust
#[sqlx::test]
async fn test_database_connection(pool: PgPool) {
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, 1);
}
```

### 2. Estructura de Test de Integración Axum
Crea una instancia de la aplicación inyectando el pool de pruebas. Usa `axum_test` o `tower::Service` para realizar peticiones.

```rust
async fn spawn_app(pool: PgPool) -> Router {
    crate::app::create_router(pool).await
}
```

### 3. Mocking de Servicios Externos
Usa `mockall` para aislar servicios como Stripe o Cloudinary y evitar llamadas reales a red durante los tests.

```rust
#[automock]
pub trait CloudinaryService {
    async fn upload(&self, data: Vec<u8>) -> Result<String, String>;
}

// En el test
let mut mock = MockCloudinaryService::new();
mock.expect_upload().returning(|_| Ok("http://test.url".into()));
```

### 4. Tests de Endpoints Protegidos (JWT)
Para testear rutas con `AuthMiddleware`, genera un JWT válido para un usuario de fixture y añádelo a la cabecera `Authorization`.

```rust
fn create_test_token(user_id: Uuid) -> String {
    let claims = Claims { sub: user_id, exp: 10000000000 };
    encode(&Header::default(), &claims, &DecodingKey::from_secret(b"secret")).unwrap()
}
```

### 5. Fixtures de Datos Reutilizables
Crea utilidades para poblar la base de datos con estados conocidos (ej: usuario logueado, anuncio creado).

### 6. Cobertura con Cargo Tarpaulin
Ejecuta `cargo tarpaulin` para generar informes de cobertura. El objetivo para Nebripop es >70% de cobertura en lógica de dominio.

---

## Ejemplos Completos de Tests

### Ejemplo 1: Registro de Usuario (Módulo `users`)
Verifica que el registro crea el usuario y hashea la contraseña correctamente.

```rust
#[sqlx::test]
async fn test_register_user_success(pool: PgPool) {
    let app = spawn_app(pool).await;
    
    let response = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"email":"test@nebripop.com","password":"secure123"}"#))
            .unwrap()
    ).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}
```

### Ejemplo 2: Crear Anuncio Protegido (Módulo `listings`)
Valida el flujo de subir un anuncio con autenticación JWT.

```rust
#[sqlx::test]
async fn test_create_listing_requires_auth(pool: PgPool) {
    let app = spawn_app(pool.clone()).await;
    let user = create_fixture_user(&pool).await;
    let token = create_test_token(user.id);

    let response = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/listings")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"title":"Coche","price":5000.0,"category":"motor"}"#))
            .unwrap()
    ).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}
```

### Ejemplo 3: Webhook de Pago Mockeado (Módulo `payments`)
Testea que el sistema reacciona al éxito de un pago confirmando la transacción.

```rust
#[sqlx::test]
async fn test_payment_webhook_updates_status(pool: PgPool) {
    let app = spawn_app(pool.clone()).await;
    let tx = create_fixture_transaction(&pool, "pi_test_123").await;

    // Simular evento de Stripe
    let body = r#"{"type":"payment_intent.succeeded","data":{"object":{"id":"pi_test_123"}}}"#;
    
    let response = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/payments/webhook")
            .header("Stripe-Signature", "t=...,v1=...") // Mock signature
            .body(Body::from(body))
            .unwrap()
    ).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    // Verificar en DB
    let updated_tx = get_transaction(&pool, tx.id).await;
    assert_eq!(updated_tx.status, "paid");
}
```

## Recomendaciones de Desarrollo
- **Tests Atómicos**: Cada test debe ser independiente y limpiar sus propios datos vía rollback o DB efímera.
- **Evitar Polling**: En tests de WebSockets o Chat, utiliza canales de sincronización en lugar de `sleep`.
- **Nomenclatura**: Usa nombres descriptivos como `test_user_cannot_delete_other_peoples_listing`.
