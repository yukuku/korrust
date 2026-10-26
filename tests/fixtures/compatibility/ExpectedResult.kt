package compatibility

data class Snapshot(val logs: List<String>) {
    companion object {
        val EMPTY = Snapshot(logs = emptyList())
    }
}

fun nullableList(): List<String>? = emptyList()
