use std::{collections::BTreeMap, time::SystemTime};

use crate::logging::{EventSpan, FirestoreEvent, LogEvent};
use tracing::{info, Level};
use tracing_subscriber::Layer;
use valuable::Valuable;

#[derive(Default)]
pub struct EspyLogsLayer;

impl<S> Layer<S> for EspyLogsLayer
where
    S: tracing::Subscriber,
    S: for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).unwrap();
        if *span.metadata().level() > Level::INFO {
            return;
        }

        let mut fields = BTreeMap::new();
        let mut visitor = JsonVisitor(&mut fields);
        attrs.record(&mut visitor);

        let mut extensions = span.extensions_mut();
        extensions.insert(EventSpan::new(
            span.name(), /*span.metadata().target()*/
        ));
        extensions.insert(StartTime(SystemTime::now()));
    }

    fn on_close(&self, id: tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span = ctx.span(&id).unwrap();
        if *span.metadata().level() > Level::INFO {
            return;
        }

        let mut extensions = span.extensions_mut();
        if let Some(mut event_span) = extensions.remove::<EventSpan>() {
            event_span.latency = match extensions.remove::<StartTime>() {
                Some(start) => SystemTime::now()
                    .duration_since(start.0)
                    .unwrap()
                    .as_millis() as u64,
                None => 0,
            };

            match span.scope().nth(1) {
                Some(parent) => {
                    let mut extensions = parent.extensions_mut();
                    if let Some(parent_event_span) = extensions.get_mut::<EventSpan>() {
                        parent_event_span.children.push(event_span);
                    }
                }
                None => {
                    if !(event_span.children.is_empty() && event_span.events.is_empty()) {
                        info!(
                            // "top log entry ==> {}",
                            // serde_json::to_string_pretty(&event_span).unwrap(),
                            entry = event_span.as_value(),
                            "log entry",
                        )
                    }
                }
            }
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = BTreeMap::new();
        let mut visitor = JsonVisitor(&mut fields);
        event.record(&mut visitor);

        if let Some(e) = fields.get("event") {
            let log: FirestoreEvent = serde_json::from_str(e.as_str().unwrap())
                .expect("Failed to prase FirestoreOp from event field.");

            if let Some(scope) = ctx.event_scope(event) {
                if let Some(span) = scope.into_iter().next() {
                    let mut extensions = span.extensions_mut();
                    if let Some(event_span) = extensions.get_mut::<EventSpan>() {
                        event_span.events.push(LogEvent::FirestoreEvent(log));
                    }
                }
            }
        }
    }
}

struct StartTime(SystemTime);

struct JsonVisitor<'a>(&'a mut BTreeMap<String, serde_json::Value>);

impl<'a> tracing::field::Visit for JsonVisitor<'a> {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.0.insert(
            field.name().to_string(),
            serde_json::json!(value.to_string()),
        );
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(
            field.name().to_string(),
            serde_json::json!(format!("{:?}", value)),
        );
    }
}
