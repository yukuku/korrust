inline fun boundary(crossinline block: () -> Int): () -> Int = { block() }
