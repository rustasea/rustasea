//! Adapter catalogue tests — the 12-provider invariant + capability matrix.

use rustavel_ai::{
    catalogue, stub_embedding, Ai, AiError, ContentPart, InProcessProvider, EMBEDDING_DIM,
    PROVIDERS, PROVIDER_COUNT,
};

/// Registry holding one adapter per catalogue provider.
fn registry() -> Ai {
    let mut ai = Ai::new();
    ai.register(InProcessProvider::openai());
    ai.register(InProcessProvider::anthropic());
    ai.register(InProcessProvider::gemini());
    ai.register(InProcessProvider::azure());
    ai.register(InProcessProvider::bedrock());
    ai.register(InProcessProvider::groq());
    ai.register(InProcessProvider::xai());
    ai.register(InProcessProvider::deepseek());
    ai.register(InProcessProvider::mistral());
    ai.register(InProcessProvider::ollama());
    ai.register(InProcessProvider::openrouter());
    ai.register(InProcessProvider::openai_compatible());
    ai
}

/// TC-M6-16 — twelve adapters available (openai..openai_compatible).
#[test]
fn twelve_providers_available() {
    assert_eq!(catalogue().len(), 12);
    assert_eq!(catalogue(), PROVIDERS.to_vec());
    let ai = registry();
    // ProviderCall builder resolves each catalogue name.
    for name in catalogue() {
        assert!(ai.provider(name).is_ok(), "missing provider {name}");
    }
    assert_eq!(PROVIDER_COUNT, 12);
}

/// Registry provider resolution by catalogue name.
#[test]
fn registry_resolves_catalogue_names() {
    let ai = registry();
    for name in catalogue() {
        assert!(ai.resolve(name).is_ok(), "missing provider {name}");
    }
}

/// INTEL-POS-006 — provider switch preserves the AiResponse shape.
#[tokio::test]
async fn provider_switch_preserves_response_shape() {
    let ai = registry();
    let openai = ai
        .provider("openai")
        .unwrap()
        .text("hello")
        .send()
        .await
        .unwrap();
    let anthropic = ai
        .provider("anthropic")
        .unwrap()
        .text("hello")
        .send()
        .await
        .unwrap();
    assert_eq!(
        openai.text.split(' ').count(),
        anthropic.text.split(' ').count()
    );
    assert!(openai.usage.is_object());
    assert!(anthropic.usage.is_object());
    assert!(openai.tool_calls.is_empty());
    assert!(anthropic.tool_calls.is_empty());
}

/// INTEL-NEG-004 — ollama×reranking and groq×files are capability denials.
#[tokio::test]
async fn unsupported_capability_matrix() {
    let ai = registry();
    let err = ai
        .provider("ollama")
        .unwrap()
        .rerank("q", vec!["a".to_string()])
        .send()
        .await
        .unwrap_err();
    match err {
        AiError::UnsupportedCapability {
            provider,
            capability,
        } => {
            assert_eq!(provider, "ollama");
            assert_eq!(capability, "reranking");
        }
        other => panic!("unexpected error: {other}"),
    }

    let err = ai
        .provider("groq")
        .unwrap()
        .files(vec![ContentPart::Text("x".to_string())])
        .send()
        .await
        .unwrap_err();
    match err {
        AiError::UnsupportedCapability {
            provider,
            capability,
        } => {
            assert_eq!(provider, "groq");
            assert_eq!(capability, "files");
        }
        other => panic!("unexpected error: {other}"),
    }
}

/// TC-M6-24 — openai embeddings advertise dimension 1536.
#[tokio::test]
async fn embedding_dim_is_1536_for_openai() {
    let ai = registry();
    let response = ai
        .provider("openai")
        .unwrap()
        .embeddings(vec!["hello".to_string()])
        .send()
        .await
        .unwrap();
    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].len(), EMBEDDING_DIM);
    assert_eq!(response.data[0].len(), 1536);
}

/// Stub embeddings are deterministic per input.
#[test]
fn stub_embedding_is_deterministic() {
    let a = stub_embedding("hello");
    let b = stub_embedding("hello");
    let c = stub_embedding("world");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.len(), EMBEDDING_DIM);
}
