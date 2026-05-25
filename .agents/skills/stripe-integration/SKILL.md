# Skill: Stripe Integration for Nebripop

Esta skill define el estándar para el procesamiento de pagos C2C en Nebripop, utilizando Stripe en modo test para asegurar transacciones seguras sin almacenar datos bancarios sensibles.

## Contexto
Según el **PRD (Módulo payments)**, Nebripop actúa como intermediario. El flujo debe ser: Reserva de fondos (PaymentIntent) -> Confirmación -> Webhook -> Actualización de estado en BD. Ningún dato de tarjeta toca nuestro servidor (PCI Compliance).

## Reglas y Ejemplos

### 1. Manejo de Importes en Céntimos
Stripe y PostgreSQL deben tratar el dinero como enteros en céntimos para evitar errores de redondeo de punto flotante.

```rust
// 29.99 EUR -> 2999 céntimos
let amount_in_cents = (price * 100.0) as i64;
```

### 2. Creación del PaymentIntent
Usa `stripe-rust` para crear la intención de pago. Incluye el `listing_id` en la metadata para rastreo.

```rust
use stripe::{Client, PaymentIntent, CreatePaymentIntent};

pub async fn create_payment_intent(client: &Client, amount: i64, listing_id: &str) -> Result<PaymentIntent, AppError> {
    let mut params = CreatePaymentIntent::new(amount, Currency::EUR);
    params.metadata = Some(vec![("listing_id".to_string(), listing_id.to_string())].into_iter().collect());
    
    let intent = PaymentIntent::create(client, params).await?;
    Ok(intent)
}
```

### 3. Endpoint de Webhook Seguro
Define un endpoint `POST /payments/webhook` que reciba el cuerpo crudo (bytes) para la verificación de firma.

```rust
pub async fn stripe_webhook(
    headers: HeaderMap,
    body: Bytes,
    Extension(stripe_client): Extension<Client>,
) -> Result<impl IntoResponse, AppError> {
    let signature = headers.get("Stripe-Signature")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let event = stripe::Webhook::construct_event(
        std::str::from_utf8(&body)?,
        signature,
        &std::env::var("STRIPE_WEBHOOK_SECRET")?
    )?;

    process_event(event).await
}
```

### 4. Verificación de Firma
Nunca proceses un webhook sin verificar la firma con el `STRIPE_WEBHOOK_SECRET`. Esto previene ataques de suplantación.

### 5. Ciclo de Vida de la Transacción en BD
Registra la transacción como `pending` al crear el intent y actualízala a `paid` solo cuando llegue el webhook `payment_intent.succeeded`.

```rust
// En el handler de webhook
if event.type_ == EventType::PaymentIntentSucceeded {
    let intent = event.data.object.as_payment_intent().unwrap();
    sqlx::query!("UPDATE transactions SET status = 'paid' WHERE stripe_payment_id = $1", intent.id)
        .execute(pool).await?;
}
```

### 6. Idempotencia de Webhooks
Stripe puede enviar el mismo webhook varias veces. Verifica si la transacción ya está en estado `paid` antes de procesar la lógica de negocio (ej: enviar notificaciones).

### 7. Uso del Modo Test
Utiliza exclusivamente tarjetas de prueba.
- **Éxito**: `4242 4242 4242 4242`.
- **Fallo (Fondos insuficientes)**: `4000 0000 0000 0002`.
- **Fallo (Tarjeta robada)**: `4000 0000 0000 0041`.

### 8. Datos Permitidos vs Prohibidos
- **PERMITIDO guardar**: `stripe_payment_id` (pi_...), `client_secret`, estado del pago, marca de la tarjeta (Visa/MC), últimos 4 dígitos.
- **PROHIBIDO guardar**: Número completo de tarjeta (PAN), CVV, Fecha de caducidad.

### 9. Manejo de Errores Detallado
Mapea los códigos de error de Stripe a mensajes amigables para el usuario en el frontend (Askama/JS).

```rust
match stripe_error {
    StripeError::CardError(e) => AppError::PaymentFailed(e.message),
    StripeError::ApiConnectionError(_) => AppError::Internal("Error de conexión con el banco"),
    _ => AppError::Internal("Error desconocido en el proceso de pago"),
}
```

### 10. Cálculo de Comisión de Plataforma
Calcula la comisión antes de mostrar el total al comprador.
```rust
const FEE_PERCENT: f64 = 0.05; // 5%
let platform_fee = (original_price * FEE_PERCENT * 100.0) as i64;
let total_amount = amount_in_cents + platform_fee;
```

## Recomendaciones de Desarrollo
- **Stripe CLI**: Para pruebas locales usa `stripe listen --forward-to localhost:3000/payments/webhook`.
- **Logs**: Registra cada evento de webhook recibido en una tabla de logs o en el sistema de trazabilidad para depurar fallos en producción.
- **Timeout**: Configura timeouts generosos para las llamadas a la API de Stripe (ej: 30s).
