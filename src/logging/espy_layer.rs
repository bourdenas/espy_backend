use std::{collections::BTreeMap, time::SystemTime};

use crate::logging::{EventSpan, LogEvent};
use tracing::{error, info, Level};
use tracing_subscriber::Layer;
use valuable::Valuable;

use super::LogRequest;

#[derive(Default)]
pub struct EspyLogsLayer {
    pub prod: bool,
    pub log_type: &'static str,
}

impl<S> Layer<S> for EspyLogsLayer
where
    S: tracing::Subscriber,
    S: for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        _attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).unwrap();
        if *span.metadata().level() > Level::INFO {
            return;
        }

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
                Some(_parent) => {
                    if let Some(root) = span.scope().from_root().next() {
                        let mut extensions = root.extensions_mut();
                        if let Some(root_event_span) = extensions.get_mut::<EventSpan>() {
                            root_event_span
                                .latencies
                                .insert(event_span.name.to_owned(), event_span.latency);
                        }
                    }
                }
                None => {
                    if self.prod {
                        if event_span.errors.is_empty() {
                            info!(
                                labels.log_type = &self.log_type,
                                labels.handler = span.name(),
                                entry = event_span.as_value(),
                                "{}",
                                event_span.request
                            );
                        } else {
                            error!(
                                labels.log_type = &self.log_type,
                                labels.handler = span.name(),
                                entry = event_span.as_value(),
                                "{}",
                                event_span.request
                            );
                        };
                    } else {
                        if event_span.errors.is_empty() {
                            info!(
                                "'{}' log entry => {}",
                                span.name(),
                                serde_json::to_string_pretty(&event_span).unwrap(),
                            )
                        } else {
                            error!(
                                "'{}' log entry => {}",
                                span.name(),
                                serde_json::to_string_pretty(&event_span).unwrap(),
                            )
                        }
                    }
                }
            }
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        if let Some(scope) = ctx.event_scope(event) {
            if let Some(span) = scope.from_root().next() {
                let mut extensions = span.extensions_mut();
                if let Some(event_span) = extensions.get_mut::<EventSpan>() {
                    let collector = FieldCollector::new(event);
                    if let Some(field) = collector.fields.get("event") {
                        if let Field::Str(encoded) = field {
                            let event: LogEvent = serde_json::from_str(encoded)
                                .expect("Failed to parse LogEvent from 'event' log field.");
                            event_span.events.add(event);
                        }
                    } else if let Some(field) = collector.fields.get("request") {
                        if let Field::Str(encoded) = field {
                            let request: LogRequest = serde_json::from_str(encoded)
                                .expect("Failed to parse LogRequest from 'request' log field.");
                            event_span.request = request;
                        }
                    } else if let Some(field) = collector.fields.get("error") {
                        if let Field::Str(error) = field {
                            event_span.errors.push(error.clone());
                        }
                    }
                }
            }
        }
    }
}

struct StartTime(SystemTime);

struct FieldCollector {
    fields: BTreeMap<&'static str, Field>,
}

enum Field {
    Float,
    Int,
    Unsigned,
    Bool,
    Str(String),
}

impl FieldCollector {
    fn new(event: &tracing::Event<'_>) -> Self {
        let mut collector = FieldCollector {
            fields: BTreeMap::new(),
        };
        event.record(&mut collector);
        collector
    }
}

impl tracing::field::Visit for FieldCollector {
    fn record_f64(&mut self, field: &tracing::field::Field, _value: f64) {
        self.fields.insert(field.name(), Field::Float);
    }

    fn record_i64(&mut self, field: &tracing::field::Field, _value: i64) {
        self.fields.insert(field.name(), Field::Int);
    }

    fn record_u64(&mut self, field: &tracing::field::Field, _value: u64) {
        self.fields.insert(field.name(), Field::Unsigned);
    }

    fn record_bool(&mut self, field: &tracing::field::Field, _value: bool) {
        self.fields.insert(field.name(), Field::Bool);
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name(), Field::Str(value.to_owned()));
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.fields
            .insert(field.name(), Field::Str(value.to_string()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name(), Field::Str(format!("{:?}", value)));
    }
}
