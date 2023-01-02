use serde::{Deserialize, Serialize};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer, Registry,
};
use tracing_tree::HierarchicalLayer;

/// Ursa Telemetry Configuration
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TelemetryConfig {
    /// Service name.
    pub name: String,
    /// Service log level.
    pub log_level: String,
    /// Service json log output.
    pub pretty_log: bool,
    /// Tokio console support.
    pub tokio_console: bool,
    /// Hierarchical log tracing.
    pub tree_tracer: bool,
    /// Chrome tracing support.
    pub chrome_tracer: bool,
    /// Jaeger tracing layer.
    pub jaeger_tracer: bool,
}

impl TelemetryConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            log_level: "INFO".to_string(),
            pretty_log: false,
            tokio_console: false,
            tree_tracer: false,
            chrome_tracer: false,
            jaeger_tracer: false,
        }
    }

    pub fn with_log_level(mut self, log_level: &str) -> Self {
        self.log_level = log_level.to_string();
        self
    }

    pub fn with_pretty_log(mut self) -> Self {
        self.pretty_log = true;
        self
    }

    pub fn with_tokio_console(mut self) -> Self {
        self.tokio_console = true;
        self
    }

    pub fn with_tree_tracer(mut self) -> Self {
        self.tree_tracer = true;
        self
    }

    pub fn with_chrome_tracer(mut self) -> Self {
        self.chrome_tracer = true;
        self
    }

    pub fn with_jaeger_tracer(mut self) -> Self {
        self.jaeger_tracer = true;
        self
    }

    pub fn init(self) -> anyhow::Result<()> {
        let mut tracing_layers = vec![];

        if self.pretty_log {
            let log_subscriber = fmt::layer()
                .pretty()
                .with_filter(EnvFilter::new(&self.log_level));
            tracing_layers.push(log_subscriber.boxed());
        }

        #[cfg(feature = "tokio-console")]
        if self.tokio_console {
            tracing_layers.push(console_subscriber::spawn().boxed());
        }

        #[cfg(feature = "tracing-tree")]
        if self.tree_tracer {
            let hierarchical_layer = HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true)
                .with_filter(EnvFilter::new(&self.log_level))
                .boxed();
            tracing_layers.push(hierarchical_layer.boxed());
        }

        #[cfg(feature = "chrome")]
        if self.chrome_tracer {
            let (chrome_layer, _guard) = tracing_chrome::ChromeLayerBuilder::new().build();
            tracing_layers.push(chrome_layer.boxed());
        }

        if self.jaeger_tracer {
            let tracer = opentelemetry_jaeger::new_agent_pipeline()
                .install_batch(opentelemetry::runtime::Tokio)?;
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            tracing_layers.push(telemetry.boxed())
        }

        Registry::default()
            .with(tracing_layers)
            .with(ErrorLayer::default())
            .init();

        Ok(())
    }

    pub fn teardown() {
        opentelemetry::global::shutdown_tracer_provider();
    }
}
