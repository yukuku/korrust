fun unsignedNeighbor(value: UInt): UInt = value + 1u
suspend fun probe(step: suspend () -> Int): Int { val x = step(); return x + 1 }