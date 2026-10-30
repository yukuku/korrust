inline fun boundary(block: () -> Int): Int { try { return block() } finally { System.nanoTime() } }
