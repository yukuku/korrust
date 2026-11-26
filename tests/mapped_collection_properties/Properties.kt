package mapped

fun keys(input: Map<String, Int>): Set<String> = input.keys
fun values(input: Map<String, Int?>): Collection<Int?> = input.values
fun entries(input: Map<String, Int>) = input.entries
fun mapSize(input: Map<String, Int>): Int = input.size
fun collectionSize(input: Collection<String>): Int = input.size
fun inherited(input: Renamed<String, Int>): Set<String> = input.keys
fun keyCount(input: Map<String, Int>): Int = input.keys.size
