fun consumer(): String {
    val swap: aliases.library.JavaTypes.Pair<Int, String> = aliases.consumer.swap()
    val nested: aliases.library.JavaTypes.Box<List<String>> = aliases.consumer.nested()
    val standard: java.util.LinkedHashMap<String, String> = aliases.consumer.standard(mapOf("key" to "copy"))
    return swap.second + nested.value[0] + standard.get("key")
}
