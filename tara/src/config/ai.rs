use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]

pub struct Ai {
    /// `None` disables the LLM feature at runtime where configuring the settings within
    /// enables it.
    pub llm: Option<Llm>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Llm {
    /// The path of the model
    pub model:                PathBuf,
    /// The context size ("memory") the model should use when evaluating a prompt. A
    /// larger context consumes more resources, but produces more consistent and coherent
    /// responses.
    pub context_token_length: Option<usize>,
    /// If `None` or invalid it will be attemtped to be infferred from the model.
    pub architecture:         Option<String>,
    /// For GGML formats that support it, mmap (memory mapped I/O) is the default.
    /// Although mmap typically improves performance, setting this value to false may be
    /// preferred in resource-constrained environments.
    pub prefer_mmap:          Option<bool>,
    /// Whether or not to use GPU support.
    pub use_gpu:              Option<bool>,
    /// The number of layers to offload to the GPU (if `use_gpu` is on).
    /// If not set, all layers will be offloaded.
    pub gpu_layers:           Option<usize>,
    /// The number of threads to use. **If this is `None` the number of *physical* cores
    /// will be automatically chosen.**
    ///
    /// Note that you should aim for a value close to the number of physical cores
    /// on the system, as this will give the best performance. This means that, for
    /// example, on a 16-core system with hyperthreading, you should set this to 16.
    ///
    /// Also note that not all cores on a system are equal, and that you may need to
    /// experiment with this value to find the optimal value for your use case. For
    /// example, Apple Silicon and modern Intel processors have "performance" and
    /// "efficiency" cores, and you may want to only use the performance cores.
    pub thread_count:         Option<usize>,
    /// Controls batch/chunk size for prompt ingestion in [InferenceSession::feed_prompt].
    ///
    /// This is the number of tokens that will be ingested at once. This is useful for
    /// trying to speed up the ingestion of prompts, as it allows for parallelization.
    /// However, you will be fundamentally limited by your machine's ability to evaluate
    /// the transformer model, so increasing the batch size will not always help.
    ///
    /// A reasonable default value is 8.
    pub batch_size:           Option<usize>,
}

impl Llm {
    #[cfg(feature = "ai")]
    pub fn architecture(&self) -> Option<llm::ModelArchitecture> {
        self.architecture.as_ref().and_then(|x| x.parse().ok())
    }
}
