-- Optional knowledge embeddings scaffold (no runtime wiring).
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
        CREATE TABLE IF NOT EXISTS knowledge_embedding_scaffold (
            id BIGSERIAL PRIMARY KEY,
            source_ref TEXT NOT NULL CHECK (char_length(source_ref) BETWEEN 1 AND 512),
            embedding vector(1536),
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );
    END IF;
END
$$;
