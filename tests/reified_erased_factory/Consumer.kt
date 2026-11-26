import fixtures.slot
fun read(): String? {
    val box = slot<List<String>>()
    box.value = listOf("ok")
    return box.value?.get(0)
}
