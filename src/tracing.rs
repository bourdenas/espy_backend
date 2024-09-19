use crate::Status;
use tracing::Level;
use tracing_stackdriver::CloudTraceConfiguration;
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub struct Tracing;

impl Tracing {
    pub fn setup(_name: &str) -> Result<(), Status> {
        match tracing_subscriber::registry()
            .with(
                // Log also to stdout.
                tracing_subscriber::fmt::Layer::new()
                    .with_writer(std::io::stdout.with_max_level(Level::WARN)),
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
