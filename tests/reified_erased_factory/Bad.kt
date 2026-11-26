import fixtures.slot
fun bad() { slot<List<String>>().value = listOf(42) }
