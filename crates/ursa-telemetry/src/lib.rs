use serde::{Deserialize, Serialize};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer, Registry,
};
use tracing_tree::HierarchicalLayer;

/// Ursa Telemetry Configuration
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TelemetryConfig {
    /// Service name.
    pub name: String,
    /// Service log level.
    pub log_level: Option<String>,
    /// Service json log output.
    pub json_log: bool,
    /// Tokio console support.
    pub tokio_console: bool,
    /// Hierarchical log tracing.
    pub tree_trace: bool,
    /// Chrome tracing support.
    pub chrome_trace: bool,
    /// Jaeger tracing layer.
    pub jaeger_trace: bool,
}

impl TelemetryConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            log_level: None,
            json_log: false,
            tokio_console: false,
            tree_trace: false,
            chrome_trace: false,
            jaeger_trace: false,
        }
    }

    pub fn with_log_level(mut self, log_level: &str) -> Self {
        self.log_level = Some(log_level.to_owned());
        self
    }

    pub fn init(self) -> anyhow::Result<()> {
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(self.log_level.unwrap_or_else(|| "info".to_string()))
        });

        let mut tracing_layers = vec![];

        #[cfg(feature = "tokio-console")]
        if self.tokio_console {
            tracing_layers.push(console_subscriber::spawn().boxed());
        }

        #[cfg(feature = "tracing-tree")]
        if self.tree_trace {
            let hierarchical_layer = HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true)
                .boxed();
            tracing_layers.push(hierarchical_layer.boxed());
        }

        #[cfg(feature = "chrome")]
        if self.chrome_trace {
            let (chrome_layer, _guard) = tracing_chrome::ChromeLayerBuilder::new().build();
            tracing_layers.push(chrome_layer.boxed());
        }

        if self.jaeger_trace {
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
