use std::{collections::BTreeMap, time::SystemTime};

use crate::{logging::EventSpan, Status};
use tracing::{level_filters::LevelFilter, Level};
use tracing_stackdriver::CloudTraceConfiguration;
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

pub struct Tracing;

impl Tracing {
    pub fn setup(_name: &str) -> Result<(), Status> {
        match tracing_subscriber::registry()
            // .with(tracing_opentelemetry::layer().with_tracer(jaeger_tracer))
            .with(EspyLogsLayer.with_filter(LevelFilter::INFO))
            .with(
                // Log also to stdout.
                tracing_subscriber::fmt::Layer::new()
                    .with_writer(std::io::stdout.with_max_level(Level::INFO)),
            )
            .try_init()
        {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("{e}");
                return Err(Status::new("Failed to setup tracing", e));
            }
        }
    }

    pub fn setup_prod(project_id: &str) -> Result<(), Status> {
        match tracing_subscriber::registry()
            .with(tracing_opentelemetry::layer())
            .with(
                tracing_stackdriver::layer()
                    .with_cloud_trace(CloudTraceConfiguration {
                        project_id: project_id.to_owned(),
                    })
                    // .with_writer(std::io::stdout.with_filter()),
                    .with_writer(std::io::stdout.with_max_level(Level::INFO)),
            )
            .try_init()
        {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("{e}");
                return Err(Status::new("Failed to setup tracing", e));
            }
        }
    }
}

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
        println!("span ==> {}", span.name());

        let mut fields = BTreeMap::new();
        let mut visitor = JsonVisitor(&mut fields);
        attrs.record(&mut visitor);

        let mut extensions = span.extensions_mut();
        if let Some(event_span) = EventSpan::create(&fields) {
            extensions.insert(StartTime(SystemTime::now()));
            extensions.insert(event_span);
        }

        let storage = CustomFieldStorage(fields);
        extensions.insert::<CustomFieldStorage>(storage);
    }

    fn on_close(&self, id: tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span = ctx.span(&id).unwrap();
        let mut extensions = span.extensions_mut();

        if let Some(mut event_span) = extensions.remove::<EventSpan>() {
            let latency = match extensions.remove::<StartTime>() {
                Some(start) => SystemTime::now()
                    .duration_since(start.0)
                    .unwrap()
                    .as_millis() as u64,
                None => 0,
            };
            event_span.latency = latency;

            match span.scope().nth(1) {
                Some(parent) => {
                    println!(
                        "log event ==> {}",
                        serde_json::to_string_pretty(&event_span).unwrap()
                    );
                    let mut extensions = parent.extensions_mut();
                    if let Some(parent_event_span) = extensions.get_mut::<EventSpan>() {
                        parent_event_span.add(span.name(), event_span);
                    }
                }
                None => println!(
                    "top log event ==> {}",
                    serde_json::to_string_pretty(&event_span).unwrap()
                ),
            }
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        // All of the span context
        let scope = ctx.event_scope(event);
        let mut spans = vec![];
        if let Some(scope) = scope {
            for span in scope.from_root() {
                let extensions = span.extensions();
                let storage = extensions.get::<CustomFieldStorage>().unwrap();
                let field_data: &BTreeMap<String, serde_json::Value> = &storage.0;
                spans.push(serde_json::json!({
                    "target": span.metadata().target(),
                    "name": span.name(),
                    "level": format!("{:?}", span.metadata().level()),
                    "fields": field_data,
                }));
            }
        }

        // The fields of the event
        let mut fields = BTreeMap::new();
        let mut visitor = JsonVisitor(&mut fields);
        event.record(&mut visitor);

        // And create our output
        let _output = serde_json::json!({
            "target": event.metadata().target(),
            "name": event.metadata().name(),
            "level": format!("{:?}", event.metadata().level()),
            "fields": fields,
            "spans": spans,
        });
        // println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }
}

struct StartTime(SystemTime);

#[derive(Debug)]
struct CustomFieldStorage(BTreeMap<String, serde_json::Value>);

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
